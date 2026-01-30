//! Voice chat translation module.
//!
//! This module provides real-time voice translation for Discord voice channels.
//! It captures audio from users, transcribes speech, translates text, and
//! optionally synthesizes translated speech back to the channel.
//!
//! ## Architecture
//!
//! ```text
//! Discord Voice Channel
//!         │
//!         ▼ (Opus audio)
//! ┌───────────────┐
//! │   Songbird    │  ← VoiceReceiveHandler
//! │   (handler)   │
//! └───────┬───────┘
//!         │
//!         ▼
//! ┌───────────────┐
//! │ AudioBuffer   │  ← Per-user ring buffers with VAD
//! │   Manager     │
//! └───────┬───────┘
//!         │ WebSocket
//!         ▼
//! ┌───────────────────────────────────────────┐
//! │       Voice Inference Service (Python)     │
//! │  STT (Whisper) → Translation → TTS        │
//! └───────────────────────┬───────────────────┘
//!                         │
//!         ┌───────────────┼───────────────┐
//!         ▼               ▼               ▼
//!    Web Client     Broadcast       Discord VC
//!   (live feed)      Manager       (TTS audio)
//! ```

pub mod bridge;
pub mod buffer;
pub mod cache;
pub mod client;
pub mod handler;
pub mod playback;
pub mod types;

pub use bridge::{spawn_voice_bridge, spawn_voice_bridge_with_threads, VoiceBridge};
pub use buffer::AudioBufferManager;
pub use cache::{CachedTranslation, CacheStats, VoiceTranscriptionCache};
pub use client::{
    ConnectionState, QueueFullStrategy, VoiceClientConfig, VoiceClientError,
    VoiceInferenceClient,
};
pub use handler::VoiceReceiveHandler;
pub use playback::{PlaybackManager, TTSPlaybackItem};
pub use types::{
    AudioPacket, AudioSegment, SpeakerInfo, TranscriptionResult, TranscriptionSegment,
    VoiceChannelState, VoiceInferenceRequest, VoiceInferenceResponse, VoiceTranslationResult,
    DISCORD_SAMPLE_RATE, OPUS_FRAME_MS, SAMPLES_PER_FRAME,
};

use dashmap::DashMap;
use songbird::Songbird;
use std::sync::Arc;
use tracing::info;

/// Voice translation manager for the entire bot.
pub struct VoiceManager {
    /// Songbird voice manager
    songbird: Arc<Songbird>,
    /// Voice inference client
    inference_client: Arc<VoiceInferenceClient>,
    /// Per-guild voice handlers
    handlers: DashMap<u64, Arc<VoiceReceiveHandler>>,
    /// Per-guild playback managers
    playback: DashMap<u64, Arc<PlaybackManager>>,
    /// Voice transcription result cache (shared across all guilds)
    cache: Arc<VoiceTranscriptionCache>,
}

impl VoiceManager {
    /// Create a new voice manager.
    pub fn new(songbird: Arc<Songbird>, config: VoiceClientConfig) -> Self {
        let inference_client = Arc::new(VoiceInferenceClient::new(config));
        // Create LRU cache with 1000 entry capacity (~10-50 MB memory)
        let cache = Arc::new(VoiceTranscriptionCache::new(1000));

        Self {
            songbird,
            inference_client,
            handlers: DashMap::new(),
            playback: DashMap::new(),
            cache,
        }
    }

    /// Get the Songbird instance.
    pub fn songbird(&self) -> Arc<Songbird> {
        self.songbird.clone()
    }

    /// Get or create handler for a guild/channel.
    pub fn get_or_create_handler(
        &self,
        guild_id: u64,
        channel_id: u64,
    ) -> Arc<VoiceReceiveHandler> {
        self.handlers
            .entry(guild_id)
            .or_insert_with(|| {
                info!(guild_id, channel_id, "Creating voice handler");
                Arc::new(VoiceReceiveHandler::new(
                    guild_id,
                    channel_id,
                    self.inference_client.clone(),
                    self.cache.clone(),
                ))
            })
            .clone()
    }

    /// Remove handler for a guild (when leaving voice).
    pub fn remove_handler(&self, guild_id: u64) {
        self.handlers.remove(&guild_id);
        self.playback.remove(&guild_id);
        info!(guild_id, "Removed voice handler");
    }

    /// Get playback manager for a guild.
    pub fn get_or_create_playback(&self, guild_id: u64) -> Arc<PlaybackManager> {
        self.playback
            .entry(guild_id)
            .or_insert_with(|| Arc::new(PlaybackManager::new()))
            .clone()
    }

    /// Check if connected to a voice channel in a guild.
    pub async fn is_connected(&self, guild_id: u64) -> bool {
        self.songbird
            .get(serenity::model::id::GuildId::new(guild_id))
            .is_some()
    }

    /// Get inference client.
    pub fn inference_client(&self) -> Arc<VoiceInferenceClient> {
        self.inference_client.clone()
    }

    /// Subscribe to voice inference results.
    pub fn subscribe_results(
        &self,
    ) -> tokio::sync::broadcast::Receiver<VoiceInferenceResponse> {
        self.inference_client.subscribe()
    }

    /// Get reference to voice transcription cache.
    pub fn cache(&self) -> Arc<VoiceTranscriptionCache> {
        self.cache.clone()
    }
}

impl std::fmt::Debug for VoiceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceManager")
            .field("active_guilds", &self.handlers.len())
            .finish()
    }
}

// Re-export serenity for convenience
use poise::serenity_prelude as serenity;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_manager_creation() {
        // Create mock Songbird instance
        let songbird = Songbird::serenity();

        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        // Verify initial state
        assert_eq!(manager.handlers.len(), 0);
        assert_eq!(manager.playback.len(), 0);
    }

    #[tokio::test]
    async fn test_voice_manager_get_or_create_handler() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        let guild_id = 123456;
        let channel_id = 789012;

        // First call should create handler
        let handler1 = manager.get_or_create_handler(guild_id, channel_id);
        assert_eq!(manager.handlers.len(), 1);

        // Second call should return same handler
        let handler2 = manager.get_or_create_handler(guild_id, channel_id);
        assert_eq!(manager.handlers.len(), 1);

        // Should be same Arc reference
        assert!(Arc::ptr_eq(&handler1, &handler2));
    }

    #[tokio::test]
    async fn test_voice_manager_remove_handler() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        let guild_id = 111222;
        let channel_id = 333444;

        // Create handler
        let _handler = manager.get_or_create_handler(guild_id, channel_id);
        assert_eq!(manager.handlers.len(), 1);

        // Remove handler
        manager.remove_handler(guild_id);
        assert_eq!(manager.handlers.len(), 0);
    }

    #[tokio::test]
    async fn test_voice_manager_get_or_create_playback() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        let guild_id = 555666;

        // First call should create playback manager
        let playback1 = manager.get_or_create_playback(guild_id);
        assert_eq!(manager.playback.len(), 1);

        // Second call should return same manager
        let playback2 = manager.get_or_create_playback(guild_id);
        assert_eq!(manager.playback.len(), 1);

        // Should be same Arc reference
        assert!(Arc::ptr_eq(&playback1, &playback2));
    }

    #[tokio::test]
    async fn test_voice_manager_inference_client_access() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        let client = manager.inference_client();

        // Should be able to clone inference client reference
        let _client2 = manager.inference_client();
    }

    #[tokio::test]
    async fn test_voice_manager_subscribe_results() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        // Should be able to subscribe to results
        let _rx = manager.subscribe_results();
    }

    #[tokio::test]
    async fn test_voice_manager_cache_access() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        let cache = manager.cache();

        // Should be able to clone cache reference
        let _cache2 = manager.cache();
    }

    #[tokio::test]
    async fn test_voice_manager_songbird_access() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(Arc::clone(&songbird), config);

        let retrieved_songbird = manager.songbird();

        // Should be same Arc reference
        assert!(Arc::ptr_eq(&songbird, &retrieved_songbird));
    }

    #[tokio::test]
    async fn test_voice_manager_debug() {
        let songbird = Songbird::serenity();
        let config = VoiceClientConfig::default();
        let manager = VoiceManager::new(songbird, config);

        // Should be able to debug print
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("VoiceManager"));
    }
}

