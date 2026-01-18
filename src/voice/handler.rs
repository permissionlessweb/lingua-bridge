//! Songbird event handler for Discord voice audio reception.

use super::buffer::AudioBufferManager;
use super::client::VoiceInferenceClient;
use super::types::{AudioPacket, VoiceChannelState};
use async_trait::async_trait;
use songbird::{
    events::context_data::VoiceTick,
    model::payload::{ClientDisconnect, Speaking},
    Event, EventContext, EventHandler,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Voice receive handler for a single guild's voice connection.
#[derive(Clone)]
pub struct VoiceReceiveHandler {
    /// Guild ID
    guild_id: u64,
    /// Voice channel ID
    channel_id: u64,
    /// Audio buffer manager
    buffer_manager: Arc<AudioBufferManager>,
    /// Voice inference client
    inference_client: Arc<VoiceInferenceClient>,
    /// Channel state (settings, speaker mappings)
    state: Arc<RwLock<VoiceChannelState>>,
}

impl VoiceReceiveHandler {
    /// Create a new voice receive handler.
    pub fn new(
        guild_id: u64,
        channel_id: u64,
        inference_client: Arc<VoiceInferenceClient>,
    ) -> Self {
        let mut state = VoiceChannelState::default();
        state.guild_id = guild_id;
        state.channel_id = channel_id;

        Self {
            guild_id,
            channel_id,
            buffer_manager: Arc::new(AudioBufferManager::new(guild_id, channel_id)),
            inference_client,
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Get reference to the buffer manager.
    pub fn buffer_manager(&self) -> Arc<AudioBufferManager> {
        self.buffer_manager.clone()
    }

    /// Get reference to the channel state.
    pub fn state(&self) -> Arc<RwLock<VoiceChannelState>> {
        self.state.clone()
    }

    /// Update channel settings.
    pub async fn update_settings(&self, target_language: String, tts_enabled: bool) {
        let mut state = self.state.write().await;
        state.target_language = target_language;
        state.tts_enabled = tts_enabled;
    }
}

#[async_trait]
impl EventHandler for VoiceReceiveHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::SpeakingStateUpdate(Speaking {
                speaking,
                ssrc,
                user_id,
                ..
            }) => {
                // Map SSRC to user ID when a user starts speaking
                if let Some(user_id) = user_id {
                    let user_id_u64: u64 = user_id.0;
                    // We don't have username here, will get from guild cache later
                    let username = format!("User-{}", user_id_u64);

                    info!(
                        ssrc = ssrc,
                        user_id = user_id_u64,
                        speaking = speaking.microphone(),
                        "Speaking state update"
                    );

                    self.buffer_manager
                        .register_speaker(*ssrc, user_id_u64, username)
                        .await;
                }
            }

            EventContext::VoiceTick(VoiceTick { speaking, .. }) => {
                // Process audio from speaking users
                for (&ssrc, data) in speaking {
                    if let Some(decoded) = &data.decoded_voice {
                        // decoded is Vec<i16> stereo, interleaved
                        // Convert to mono by averaging channels
                        let mono: Vec<i16> = decoded
                            .chunks(2)
                            .map(|chunk| {
                                if chunk.len() == 2 {
                                    ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16
                                } else {
                                    chunk[0]
                                }
                            })
                            .collect();

                        let packet = AudioPacket {
                            ssrc,
                            user_id: None, // Will be resolved by buffer manager
                            username: None,
                            samples: mono,
                            timestamp: std::time::Instant::now(),
                            sequence: 0,
                        };

                        // Push to buffer and check if we have a complete segment
                        if let Some(segment) = self.buffer_manager.push_audio(packet).await {
                            let state = self.state.read().await;
                            let target_lang = state.target_language.clone();
                            let tts_enabled = state.tts_enabled;
                            drop(state);

                            // Send to inference service
                            if let Err(e) = self
                                .inference_client
                                .send_audio(segment, &target_lang, tts_enabled)
                                .await
                            {
                                warn!(error = %e, "Failed to send audio to inference");
                            }
                        }
                    }
                }

                // Check for timeout flushes periodically
                // (This happens every voice tick, which is ~20ms)
                let segments = self.buffer_manager.check_timeouts().await;
                for segment in segments {
                    let state = self.state.read().await;
                    let target_lang = state.target_language.clone();
                    let tts_enabled = state.tts_enabled;
                    drop(state);

                    if let Err(e) = self
                        .inference_client
                        .send_audio(segment, &target_lang, tts_enabled)
                        .await
                    {
                        warn!(error = %e, "Failed to send timeout audio to inference");
                    }
                }
            }

            EventContext::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                info!(user_id = user_id.0, "User disconnected from voice");

                // Flush any remaining audio for this user
                // Note: We don't have SSRC here, so we'll rely on timeout flush
            }

            _ => {}
        }

        None
    }
}

impl std::fmt::Debug for VoiceReceiveHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceReceiveHandler")
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .finish()
    }
}
