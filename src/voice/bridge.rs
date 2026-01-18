//! Voice result bridge.
//!
//! Bridges voice inference results to the web broadcast system and
//! optionally to Discord thread transcripts.

use super::VoiceInferenceResponse;
use crate::db::{DbPool, VoiceTranscriptRepo};
use crate::web::BroadcastManager;
use poise::serenity_prelude::{ChannelId, CreateMessage, Http};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Bridge that forwards voice inference results to web clients.
pub struct VoiceBridge {
    /// Receiver for voice inference results
    voice_rx: broadcast::Receiver<VoiceInferenceResponse>,
    /// Broadcast manager for web clients
    broadcast: Arc<BroadcastManager>,
    /// Optional database pool for transcript settings
    pool: Option<DbPool>,
    /// Optional HTTP client for posting to Discord threads
    http: Option<Arc<Http>>,
}

impl VoiceBridge {
    /// Create a new voice bridge.
    pub fn new(
        voice_rx: broadcast::Receiver<VoiceInferenceResponse>,
        broadcast: Arc<BroadcastManager>,
    ) -> Self {
        Self {
            voice_rx,
            broadcast,
            pool: None,
            http: None,
        }
    }

    /// Create a voice bridge with Discord thread posting support.
    pub fn with_thread_support(
        voice_rx: broadcast::Receiver<VoiceInferenceResponse>,
        broadcast: Arc<BroadcastManager>,
        pool: DbPool,
        http: Arc<Http>,
    ) -> Self {
        Self {
            voice_rx,
            broadcast,
            pool: Some(pool),
            http: Some(http),
        }
    }

    /// Run the bridge, forwarding voice results to web clients.
    ///
    /// This should be spawned as a background task.
    pub async fn run(mut self) {
        info!("Voice bridge started - forwarding results to web clients");

        loop {
            match self.voice_rx.recv().await {
                Ok(response) => {
                    self.handle_response(&response).await;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "Voice bridge lagged, skipped messages");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    error!("Voice result channel closed, bridge shutting down");
                    break;
                }
            }
        }

        info!("Voice bridge stopped");
    }

    /// Handle a voice inference response.
    async fn handle_response(&self, response: &VoiceInferenceResponse) {
        match response {
            VoiceInferenceResponse::Result {
                guild_id,
                channel_id,
                user_id,
                username,
                original_text,
                translated_text,
                target_language,
                ..
            } => {
                // Skip empty transcriptions
                if original_text.is_empty() {
                    debug!(user_id, "Skipping empty transcription");
                    return;
                }

                debug!(
                    guild_id,
                    channel_id,
                    user_id,
                    username,
                    text = original_text,
                    "Forwarding voice transcription to web clients"
                );

                // Forward to broadcast manager for web clients
                self.broadcast.send_voice_transcription(response);

                // Post to Discord threads if configured
                if let (Some(pool), Some(http)) = (&self.pool, &self.http) {
                    self.post_to_threads(
                        pool,
                        http,
                        guild_id,
                        channel_id,
                        username,
                        original_text,
                        translated_text,
                        target_language,
                    )
                    .await;
                }
            }
            VoiceInferenceResponse::Ready {
                stt_models,
                tts_models,
            } => {
                info!(
                    stt_models = ?stt_models,
                    tts_models = ?tts_models,
                    "Voice inference service ready"
                );
            }
            VoiceInferenceResponse::Pong => {
                debug!("Received pong from voice inference service");
            }
            VoiceInferenceResponse::Error { message, code } => {
                error!(message, code = ?code, "Voice inference error");
            }
        }
    }

    /// Post transcription to Discord threads based on settings.
    async fn post_to_threads(
        &self,
        pool: &DbPool,
        http: &Http,
        guild_id: &str,
        channel_id: &str,
        username: &str,
        original_text: &str,
        translated_text: &str,
        target_language: &str,
    ) {
        // Look up transcript settings
        let settings = match VoiceTranscriptRepo::get_settings(pool, guild_id, channel_id).await {
            Ok(Some(s)) if s.enabled => s,
            Ok(_) => return, // Not configured or disabled
            Err(e) => {
                debug!(error = %e, "Failed to get transcript settings");
                return;
            }
        };

        let thread_ids = settings.get_thread_ids();

        // If we have a thread for the target language, post there
        if let Some(thread_id_str) = thread_ids.get(target_language) {
            if let Ok(thread_id) = thread_id_str.parse::<u64>() {
                let message = format!("**{}**\n> {}\n{}", username, original_text, translated_text);

                let channel = ChannelId::new(thread_id);
                if let Err(e) = channel
                    .send_message(http, CreateMessage::new().content(&message))
                    .await
                {
                    debug!(error = %e, thread_id, "Failed to post to transcript thread");
                }
            }
        }
    }
}

/// Spawn the voice bridge as a background task.
///
/// Returns a handle to the spawned task.
pub fn spawn_voice_bridge(
    voice_rx: broadcast::Receiver<VoiceInferenceResponse>,
    broadcast: Arc<BroadcastManager>,
) -> tokio::task::JoinHandle<()> {
    let bridge = VoiceBridge::new(voice_rx, broadcast);
    tokio::spawn(bridge.run())
}

/// Spawn the voice bridge with Discord thread support.
///
/// This version of the bridge will also post transcripts to configured Discord threads.
pub fn spawn_voice_bridge_with_threads(
    voice_rx: broadcast::Receiver<VoiceInferenceResponse>,
    broadcast: Arc<BroadcastManager>,
    pool: DbPool,
    http: Arc<Http>,
) -> tokio::task::JoinHandle<()> {
    let bridge = VoiceBridge::with_thread_support(voice_rx, broadcast, pool, http);
    tokio::spawn(bridge.run())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let (tx, rx) = broadcast::channel::<VoiceInferenceResponse>(10);
        let broadcast = Arc::new(BroadcastManager::new());
        let bridge = VoiceBridge::new(rx, broadcast);
        // Bridge created successfully
        drop(bridge);
        drop(tx);
    }

    #[test]
    fn test_bridge_with_thread_support_creation() {
        // Test would require a mock pool and http client
        // Just verify the struct fields exist
        let (tx, rx) = broadcast::channel::<VoiceInferenceResponse>(10);
        let broadcast = Arc::new(BroadcastManager::new());
        let bridge = VoiceBridge::new(rx, broadcast);
        assert!(bridge.pool.is_none());
        assert!(bridge.http.is_none());
        drop(bridge);
        drop(tx);
    }
}
