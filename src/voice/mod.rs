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
pub mod client;
pub mod handler;
pub mod playback;
pub mod types;

pub use bridge::{spawn_voice_bridge, spawn_voice_bridge_with_threads, VoiceBridge};
pub use buffer::AudioBufferManager;
pub use client::{ConnectionState, VoiceClientConfig, VoiceClientError, VoiceInferenceClient};
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
}

impl VoiceManager {
    /// Create a new voice manager.
    pub fn new(songbird: Arc<Songbird>, config: VoiceClientConfig) -> Self {
        let inference_client = Arc::new(VoiceInferenceClient::new(config));

        Self {
            songbird,
            inference_client,
            handlers: DashMap::new(),
            playback: DashMap::new(),
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
