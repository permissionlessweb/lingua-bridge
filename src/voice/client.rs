//! WebSocket client for voice inference service.

use super::types::{AudioSegment, VoiceInferenceRequest, VoiceInferenceResponse};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Audio segment bundled with translation config for sending to inference.
#[derive(Debug, Clone)]
struct AudioRequest {
    segment: AudioSegment,
    target_language: String,
    generate_tts: bool,
    /// Audio hash for cache correlation (computed from samples)
    audio_hash: u64,
}

/// Strategy for handling full audio queue (backpressure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueFullStrategy {
    /// Drop oldest audio (keep recent audio) - best for real-time voice
    DropOldest,
    /// Drop new audio (preserve old audio in queue)
    DropNewest,
    /// Block until space available (risks deadlock, use with caution)
    Block,
}

/// Connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Voice inference client configuration.
#[derive(Debug, Clone)]
pub struct VoiceClientConfig {
    /// WebSocket URL for voice inference service
    pub url: String,
    /// Reconnection delay
    pub reconnect_delay: Duration,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Request timeout
    pub request_timeout: Duration,
    /// Ping interval (for detecting dead connections)
    pub ping_interval: Duration,
    /// Maximum audio queue size before backpressure kicks in
    pub max_queue_size: usize,
    /// Strategy for handling full queue
    pub queue_full_strategy: QueueFullStrategy,
}

impl Default for VoiceClientConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8001/voice".to_string(),
            reconnect_delay: Duration::from_secs(2),
            max_reconnect_attempts: 10,
            request_timeout: Duration::from_secs(30),
            // Reduced from 30s to 10s for faster dead connection detection
            ping_interval: Duration::from_secs(10),
            // 500 entries = ~10 seconds of audio at 1.5s streaming chunks
            max_queue_size: 500,
            // Drop newest for real-time voice (old audio is already stale)
            queue_full_strategy: QueueFullStrategy::DropNewest,
        }
    }
}

/// WebSocket client for voice inference.
pub struct VoiceInferenceClient {
    config: VoiceClientConfig,
    state: Arc<RwLock<ConnectionState>>,
    /// Channel to send audio requests (segment + config) for processing
    audio_tx: mpsc::Sender<AudioRequest>,
    /// Channel to receive transcription results
    _result_rx: broadcast::Receiver<VoiceInferenceResponse>,
    /// Broadcast sender for results (shared with handler)
    result_tx: broadcast::Sender<VoiceInferenceResponse>,
}

impl VoiceInferenceClient {
    /// Create a new voice inference client.
    pub fn new(config: VoiceClientConfig) -> Self {
        // Use configured queue size (with backpressure handling)
        let (audio_tx, audio_rx) = mpsc::channel(config.max_queue_size);
        let (result_tx, _result_rx) = broadcast::channel(100);

        let client = Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            audio_tx,
            _result_rx,
            result_tx: result_tx.clone(),
        };

        // Spawn connection handler
        let state = client.state.clone();
        tokio::spawn(connection_handler(config, audio_rx, result_tx, state));

        client
    }

    /// Get current connection state.
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Send audio segment for processing.
    ///
    /// Handles backpressure according to the configured strategy.
    ///
    /// The audio_hash is used to correlate responses with requests for caching.
    pub async fn send_audio(
        &self,
        segment: AudioSegment,
        target_language: &str,
        generate_tts: bool,
        audio_hash: u64,
    ) -> Result<(), VoiceClientError> {
        if !self.is_connected().await {
            return Err(VoiceClientError::NotConnected);
        }

        // Package segment with config and audio hash for cache correlation
        let req = AudioRequest {
            segment,
            target_language: target_language.to_string(),
            generate_tts,
            audio_hash,
        };

        // Try non-blocking send first
        match self.audio_tx.try_send(req) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(req)) => {
                // Queue is full - handle based on strategy
                match self.config.queue_full_strategy {
                    QueueFullStrategy::DropNewest => {
                        warn!(
                            queue_size = self.config.max_queue_size,
                            "Audio queue full, dropping newest segment (backpressure)"
                        );
                        Err(VoiceClientError::QueueFull)
                    }
                    QueueFullStrategy::DropOldest => {
                        // In production, implement a bounded queue with pop_front capability
                        // For now, just drop new and log
                        // TODO: Implement proper oldest-drop with custom queue
                        warn!(
                            queue_size = self.config.max_queue_size,
                            "Audio queue full, dropping segment (backpressure)"
                        );
                        Err(VoiceClientError::QueueFull)
                    }
                    QueueFullStrategy::Block => {
                        // Fall back to blocking send (risks deadlock but preserves all audio)
                        warn!(
                            queue_size = self.config.max_queue_size,
                            "Audio queue full, blocking until space available"
                        );
                        self.audio_tx
                            .send(req)
                            .await
                            .map_err(|_| VoiceClientError::ChannelClosed)?;
                        Ok(())
                    }
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(VoiceClientError::ChannelClosed),
        }
    }

    /// Subscribe to transcription results.
    pub fn subscribe(&self) -> broadcast::Receiver<VoiceInferenceResponse> {
        self.result_tx.subscribe()
    }

    /// Broadcast a cached result (for cache hits).
    ///
    /// This allows cached responses to flow through the same result channel
    /// as live inference responses, ensuring consistent handling.
    pub async fn broadcast_cached_result(
        &self,
        response: VoiceInferenceResponse,
    ) -> Result<(), VoiceClientError> {
        self.result_tx
            .send(response)
            .map_err(|_| VoiceClientError::BroadcastFailed)?;
        Ok(())
    }
}

/// Connection handler task.
async fn connection_handler(
    config: VoiceClientConfig,
    mut audio_rx: mpsc::Receiver<AudioRequest>,
    result_tx: broadcast::Sender<VoiceInferenceResponse>,
    state: Arc<RwLock<ConnectionState>>,
) {
    let mut reconnect_attempts = 0;

    loop {
        *state.write().await = ConnectionState::Connecting;
        info!(url = %config.url, "Connecting to voice inference service");

        match connect_async(&config.url).await {
            Ok((ws_stream, _response)) => {
                *state.write().await = ConnectionState::Connected;
                reconnect_attempts = 0;
                info!("Connected to voice inference service");

                let (mut write, mut read) = ws_stream.split();

                // Spawn reader task
                let result_tx_clone = result_tx.clone();
                let reader_handle = tokio::spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                match serde_json::from_str::<VoiceInferenceResponse>(&text) {
                                    Ok(response) => {
                                        debug!(?response, "Received voice inference response");
                                        let _ = result_tx_clone.send(response);
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "Failed to parse voice response");
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                info!("Voice inference connection closed");
                                break;
                            }
                            Ok(Message::Ping(_data)) => {
                                debug!("Received ping");
                                // Pong is handled automatically by tungstenite
                            }
                            Err(e) => {
                                error!(error = %e, "WebSocket read error");
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                // Process outgoing audio
                let mut ping_interval = tokio::time::interval(config.ping_interval);

                loop {
                    tokio::select! {
                        Some(req) = audio_rx.recv() => {
                            let segment = &req.segment;

                            // Use binary WebSocket frames instead of base64 text
                            // Format: JSON header + raw PCM data
                            let header = VoiceInferenceRequest::Audio {
                                guild_id: segment.guild_id.to_string(),
                                channel_id: segment.channel_id.to_string(),
                                user_id: segment.user_id.to_string(),
                                username: segment.username.clone(),
                                audio_base64: String::new(), // Placeholder, will send binary
                                sample_rate: super::types::DISCORD_SAMPLE_RATE,
                                target_language: req.target_language.clone(),
                                generate_tts: req.generate_tts,
                                audio_hash: req.audio_hash, // For cache correlation
                            };

                            // Serialize header as JSON
                            let header_json = serde_json::to_string(&header)
                                .expect("Failed to serialize request");
                            let header_bytes = header_json.as_bytes();

                            // Build binary message: [4-byte header length][header JSON][raw PCM i16 samples]
                            let header_len = header_bytes.len() as u32;
                            let mut binary_msg = Vec::with_capacity(
                                4 + header_bytes.len() + segment.samples.len() * 2
                            );
                            binary_msg.extend_from_slice(&header_len.to_le_bytes());
                            binary_msg.extend_from_slice(header_bytes);
                            // Raw PCM samples (no base64 encoding = 33% bandwidth savings)
                            for sample in &segment.samples {
                                binary_msg.extend_from_slice(&sample.to_le_bytes());
                            }

                            if let Err(e) = write.send(Message::Binary(binary_msg)).await {
                                error!(error = %e, "Failed to send audio to inference");
                                break;
                            }

                            debug!(
                                user_id = segment.user_id,
                                duration_ms = segment.duration().as_millis(),
                                samples = segment.samples.len(),
                                "Sent audio to inference service (binary)"
                            );
                        }

                        _ = ping_interval.tick() => {
                            let ping = serde_json::to_string(&VoiceInferenceRequest::Ping)
                                .expect("Failed to serialize ping");
                            if let Err(e) = write.send(Message::Text(ping.into())).await {
                                warn!(error = %e, "Failed to send ping");
                                break;
                            }
                        }
                    }
                }

                // Connection lost, abort reader
                reader_handle.abort();
            }
            Err(e) => {
                error!(error = %e, "Failed to connect to voice inference service");
            }
        }

        // Reconnection logic
        *state.write().await = ConnectionState::Reconnecting;
        reconnect_attempts += 1;

        if reconnect_attempts >= config.max_reconnect_attempts {
            error!(
                attempts = reconnect_attempts,
                "Max reconnection attempts reached, giving up"
            );
            *state.write().await = ConnectionState::Disconnected;
            break;
        }

        let delay = config.reconnect_delay * reconnect_attempts;
        warn!(
            attempts = reconnect_attempts,
            delay_secs = delay.as_secs(),
            "Reconnecting to voice inference service"
        );
        tokio::time::sleep(delay).await;
    }
}

/// Voice client errors.
#[derive(Debug, thiserror::Error)]
pub enum VoiceClientError {
    #[error("Not connected to voice inference service")]
    NotConnected,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Request timeout")]
    Timeout,

    #[error("Audio queue full (backpressure triggered)")]
    QueueFull,

    #[error("Failed to broadcast cached result")]
    BroadcastFailed,

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl std::fmt::Debug for VoiceInferenceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceInferenceClient")
            .field("url", &self.config.url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = VoiceClientConfig::default();
        assert_eq!(config.url, "ws://localhost:8001/voice");
        assert_eq!(config.max_reconnect_attempts, 10);
    }
}
