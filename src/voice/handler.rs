//! Songbird event handler for Discord voice audio reception.

use super::buffer::AudioBufferManager;
use super::cache::VoiceTranscriptionCache;
use super::client::VoiceInferenceClient;
use super::types::{AudioPacket, AudioSegment, VoiceChannelState};
use async_trait::async_trait;
use songbird::{
    events::context_data::VoiceTick,
    model::payload::{ClientDisconnect, Speaking},
    Event, EventContext, EventHandler,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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
    /// Voice transcription cache (shared across guilds)
    cache: Arc<VoiceTranscriptionCache>,
}

impl VoiceReceiveHandler {
    /// Create a new voice receive handler.
    pub fn new(
        guild_id: u64,
        channel_id: u64,
        inference_client: Arc<VoiceInferenceClient>,
        cache: Arc<VoiceTranscriptionCache>,
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
            cache,
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
    pub async fn update_settings(&self, target_language: Arc<str>, tts_enabled: bool) {
        let mut state = self.state.write().await;
        state.target_language = target_language;
        state.tts_enabled = tts_enabled;
    }

    /// Process audio segment: check cache first, send to inference if miss.
    async fn process_segment(
        &self,
        segment: AudioSegment,
        target_lang: Arc<str>,
        tts_enabled: bool,
    ) {
        // Check cache first (hash audio samples)
        let audio_hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

        if let Some(cached_response) = self.cache.get(audio_hash, &target_lang).await {
            // Cache hit! No need to call inference service
            debug!(
                user_id = segment.user_id,
                audio_hash,
                duration_ms = segment.duration().as_millis(),
                "Using cached translation (skipping inference)"
            );

            // Re-broadcast cached response to inference result channel
            // This allows the bridge to forward it to web clients and Discord threads
            if let Err(e) = self.inference_client.broadcast_cached_result(cached_response).await {
                warn!(error = %e, "Failed to broadcast cached result");
            }

            return;
        }

        // Cache miss - send to inference (pass audio_hash for response correlation)
        if let Err(e) = self
            .inference_client
            .send_audio(segment, &target_lang, tts_enabled, audio_hash)
            .await
        {
            warn!(error = %e, "Failed to send audio to inference");
        }

        // NOTE: When responses come back from inference, cache them in the response handler.
        // The audio_hash is tracked through the request so we can correlate the response.
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
                            // Read config (Arc clone is cheap - just atomic increment)
                            let state = self.state.read().await;
                            let target_lang = Arc::clone(&state.target_language);
                            let tts_enabled = state.tts_enabled;
                            // Lock released here automatically

                            // Process segment (checks cache, sends to inference if needed)
                            self.process_segment(segment, target_lang, tts_enabled).await;
                        }
                    }
                }

                // Check for timeout flushes periodically
                // (This happens every voice tick, which is ~20ms)
                let segments = self.buffer_manager.check_timeouts().await;
                if !segments.is_empty() {
                    // Read config once (Arc clone is cheap)
                    let state = self.state.read().await;
                    let target_lang = Arc::clone(&state.target_language);
                    let tts_enabled = state.tts_enabled;
                    // Lock released here automatically

                    // Process all timeout segments (checks cache, sends to inference if needed)
                    for segment in segments {
                        self.process_segment(segment, Arc::clone(&target_lang), tts_enabled).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::client::VoiceClientConfig;
    use crate::voice::types::VoiceInferenceResponse;

    #[tokio::test]
    async fn test_handler_creation() {
        let config = VoiceClientConfig::default();
        let client = Arc::new(VoiceInferenceClient::new(config));
        let cache = Arc::new(VoiceTranscriptionCache::new(100));

        let handler = VoiceReceiveHandler::new(123456, 789012, client, cache);

        assert_eq!(handler.guild_id, 123456);
        assert_eq!(handler.channel_id, 789012);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let config = VoiceClientConfig::default();
        let client = Arc::new(VoiceInferenceClient::new(config));
        let cache = Arc::new(VoiceTranscriptionCache::new(100));

        let handler = VoiceReceiveHandler::new(111, 222, client, cache);

        // Update settings
        handler
            .update_settings(Arc::from("fr"), true)
            .await;

        // Verify settings updated
        let state = handler.state.read().await;
        assert_eq!(state.target_language.as_ref(), "fr");
        assert_eq!(state.tts_enabled, true);
    }

    #[tokio::test]
    async fn test_process_segment_cache_miss() {
        let config = VoiceClientConfig {
            url: "ws://127.0.0.1:9999".to_string(), // Non-existent server
            reconnect_delay: std::time::Duration::from_secs(100),
            max_reconnect_attempts: 0, // Don't reconnect
            ..Default::default()
        };
        let client = Arc::new(VoiceInferenceClient::new(config));
        let cache = Arc::new(VoiceTranscriptionCache::new(100));

        let handler = VoiceReceiveHandler::new(333, 444, client, cache.clone());

        // Create test segment
        let now = std::time::Instant::now();
        let segment = AudioSegment {
            user_id: 555,
            username: "TestUser".to_string(),
            guild_id: 333,
            channel_id: 444,
            samples: vec![100, 200, 300],
            start_time: now,
            end_time: now + std::time::Duration::from_millis(100),
        };

        let audio_hash = VoiceTranscriptionCache::hash_audio(&segment.samples);

        // Verify cache miss
        assert!(cache.get(audio_hash, &Arc::from("en")).await.is_none());

        // Process segment (will try to send to non-existent server, but won't panic)
        handler
            .process_segment(segment, Arc::from("en"), false)
            .await;

        // Verify cache still empty (response never came back)
        assert!(cache.get(audio_hash, &Arc::from("en")).await.is_none());

        // Verify cache stats
        // Note: We made 3 cache get() calls: one in process_segment, two in our test checks
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 3);
    }

    #[tokio::test]
    async fn test_process_segment_cache_hit() {
        let config = VoiceClientConfig::default();
        let client = Arc::new(VoiceInferenceClient::new(config));
        let cache = Arc::new(VoiceTranscriptionCache::new(100));

        let handler = VoiceReceiveHandler::new(666, 777, client, cache.clone());

        // Pre-populate cache
        let samples = vec![400, 500, 600];
        let audio_hash = VoiceTranscriptionCache::hash_audio(&samples);
        let target_lang = Arc::from("ja");

        let cached_response = VoiceInferenceResponse::Result {
            guild_id: "666".to_string(),
            channel_id: "777".to_string(),
            user_id: "888".to_string(),
            username: "CachedUser".to_string(),
            original_text: "cached transcription".to_string(),
            translated_text: "キャッシュされた翻訳".to_string(),
            source_language: "en".to_string(),
            target_language: "ja".to_string(),
            tts_audio: None,
            latency_ms: 100,
            audio_hash,
        };

        cache
            .put(audio_hash, Arc::clone(&target_lang), cached_response)
            .await;

        // Verify cache hit
        assert!(cache.get(audio_hash, &target_lang).await.is_some());

        // Create segment with same samples
        let now = std::time::Instant::now();
        let segment = AudioSegment {
            user_id: 888,
            username: "TestUser".to_string(),
            guild_id: 666,
            channel_id: 777,
            samples: samples.clone(),
            start_time: now,
            end_time: now + std::time::Duration::from_millis(100),
        };

        // Process segment (should hit cache, not send to inference)
        handler
            .process_segment(segment, Arc::clone(&target_lang), false)
            .await;

        // Verify cache hit (one more from process_segment)
        let stats = cache.stats();
        assert_eq!(stats.hits, 2); // One from our get() check, one from process_segment()
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_buffer_manager_access() {
        let config = VoiceClientConfig::default();
        let client = Arc::new(VoiceInferenceClient::new(config));
        let cache = Arc::new(VoiceTranscriptionCache::new(100));

        let handler = VoiceReceiveHandler::new(999, 111, client, cache);

        let buffer_manager = handler.buffer_manager();

        // Verify we can access buffer manager (it's an Arc, so we can clone it)
        assert!(Arc::strong_count(&buffer_manager) >= 1);
    }

    #[tokio::test]
    async fn test_state_access() {
        let config = VoiceClientConfig::default();
        let client = Arc::new(VoiceInferenceClient::new(config));
        let cache = Arc::new(VoiceTranscriptionCache::new(100));

        let handler = VoiceReceiveHandler::new(222, 333, client, cache);

        let state = handler.state();
        let state_guard = state.read().await;

        assert_eq!(state_guard.guild_id, 222);
        assert_eq!(state_guard.channel_id, 333);
    }
}

