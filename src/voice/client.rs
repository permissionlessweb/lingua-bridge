//! WebSocket client for voice inference service.

use super::types::{AudioSegment, VoiceInferenceRequest, VoiceInferenceResponse};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

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
    /// Ping interval
    pub ping_interval: Duration,
}

impl Default for VoiceClientConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8001/voice".to_string(),
            reconnect_delay: Duration::from_secs(2),
            max_reconnect_attempts: 10,
            request_timeout: Duration::from_secs(30),
            ping_interval: Duration::from_secs(30),
        }
    }
}

/// WebSocket client for voice inference.
pub struct VoiceInferenceClient {
    config: VoiceClientConfig,
    state: Arc<RwLock<ConnectionState>>,
    /// Channel to send audio segments for processing
    audio_tx: mpsc::Sender<AudioSegment>,
    /// Channel to receive transcription results
    _result_rx: broadcast::Receiver<VoiceInferenceResponse>,
    /// Broadcast sender for results (shared with handler)
    result_tx: broadcast::Sender<VoiceInferenceResponse>,
}

impl VoiceInferenceClient {
    /// Create a new voice inference client.
    pub fn new(config: VoiceClientConfig) -> Self {
        let (audio_tx, audio_rx) = mpsc::channel(100);
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
    pub async fn send_audio(
        &self,
        segment: AudioSegment,
        _target_language: &str,
        _generate_tts: bool,
    ) -> Result<(), VoiceClientError> {
        if !self.is_connected().await {
            return Err(VoiceClientError::NotConnected);
        }

        // Send via internal channel to connection handler
        // The connection handler will convert to WebSocket message
        self.audio_tx
            .send(segment)
            .await
            .map_err(|_| VoiceClientError::ChannelClosed)?;

        Ok(())
    }

    /// Subscribe to transcription results.
    pub fn subscribe(&self) -> broadcast::Receiver<VoiceInferenceResponse> {
        self.result_tx.subscribe()
    }
}

/// Connection handler task.
async fn connection_handler(
    config: VoiceClientConfig,
    mut audio_rx: mpsc::Receiver<AudioSegment>,
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
                        Some(segment) = audio_rx.recv() => {
                            // Convert to request
                            let audio_bytes: Vec<u8> = segment
                                .samples
                                .iter()
                                .flat_map(|s| s.to_le_bytes())
                                .collect();
                            let audio_base64 = BASE64.encode(&audio_bytes);

                            let request = VoiceInferenceRequest::Audio {
                                guild_id: segment.guild_id.to_string(),
                                channel_id: segment.channel_id.to_string(),
                                user_id: segment.user_id.to_string(),
                                username: segment.username.clone(),
                                audio_base64,
                                sample_rate: super::types::DISCORD_SAMPLE_RATE,
                                target_language: "en".to_string(), // TODO: get from channel config
                                generate_tts: false, // TODO: get from channel config
                            };

                            let msg = serde_json::to_string(&request)
                                .expect("Failed to serialize request");

                            if let Err(e) = write.send(Message::Text(msg.into())).await {
                                error!(error = %e, "Failed to send audio to inference");
                                break;
                            }

                            debug!(
                                user_id = segment.user_id,
                                duration_ms = segment.duration().as_millis(),
                                "Sent audio to inference service"
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
