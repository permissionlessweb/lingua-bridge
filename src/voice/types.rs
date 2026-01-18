//! Shared types for voice translation pipeline.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Discord SSRC (Synchronization Source) identifier for a speaker.
pub type Ssrc = u32;

/// Audio sample rate in Hz (Discord uses 48kHz).
pub const DISCORD_SAMPLE_RATE: u32 = 48000;

/// Opus frame duration in milliseconds (Discord uses 20ms frames).
pub const OPUS_FRAME_MS: u32 = 20;

/// Samples per Opus frame at 48kHz.
pub const SAMPLES_PER_FRAME: usize = (DISCORD_SAMPLE_RATE * OPUS_FRAME_MS / 1000) as usize;

/// Raw audio packet from Discord voice.
#[derive(Debug, Clone)]
pub struct AudioPacket {
    /// Discord SSRC identifying the speaker
    pub ssrc: Ssrc,
    /// Discord user ID (resolved from SSRC)
    pub user_id: Option<u64>,
    /// Username (if available)
    pub username: Option<String>,
    /// Decoded PCM audio samples (i16, mono, 48kHz)
    pub samples: Vec<i16>,
    /// Timestamp when this packet was received
    pub timestamp: Instant,
    /// Sequence number for ordering
    pub sequence: u16,
}

impl AudioPacket {
    /// Duration of this audio packet.
    pub fn duration(&self) -> Duration {
        let sample_count = self.samples.len();
        Duration::from_secs_f64(sample_count as f64 / DISCORD_SAMPLE_RATE as f64)
    }
}

/// Aggregated audio segment ready for transcription.
#[derive(Debug, Clone)]
pub struct AudioSegment {
    /// Discord user ID
    pub user_id: u64,
    /// Username
    pub username: String,
    /// Guild ID
    pub guild_id: u64,
    /// Voice channel ID
    pub channel_id: u64,
    /// PCM audio samples (i16, mono, 48kHz)
    pub samples: Vec<i16>,
    /// Start timestamp
    pub start_time: Instant,
    /// End timestamp
    pub end_time: Instant,
}

impl AudioSegment {
    /// Duration of this audio segment.
    pub fn duration(&self) -> Duration {
        self.end_time.duration_since(self.start_time)
    }

    /// Convert samples to f32 normalized [-1.0, 1.0].
    pub fn samples_f32(&self) -> Vec<f32> {
        self.samples.iter().map(|&s| s as f32 / 32768.0).collect()
    }

    /// Convert samples to bytes (little-endian i16).
    pub fn samples_bytes(&self) -> Bytes {
        let mut bytes = Vec::with_capacity(self.samples.len() * 2);
        for sample in &self.samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Bytes::from(bytes)
    }
}

/// Speaker information for diarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerInfo {
    /// Unique speaker label from diarization
    pub speaker_id: String,
    /// Discord user ID if matched
    pub user_id: Option<u64>,
    /// Discord username if available
    pub username: Option<String>,
    /// Speaker embedding for voice matching (optional)
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
}

/// Transcription segment with timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Transcribed text
    pub text: String,
    /// Start time in seconds
    pub start: f64,
    /// End time in seconds
    pub end: f64,
    /// Speaker information
    pub speaker: Option<SpeakerInfo>,
    /// Confidence score (0.0-1.0)
    pub confidence: Option<f32>,
    /// Detected language
    pub language: Option<String>,
}

/// Complete transcription result from voice inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Full transcribed text
    pub text: String,
    /// Individual segments with timing
    pub segments: Vec<TranscriptionSegment>,
    /// Detected source language
    pub source_language: String,
    /// Audio duration in seconds
    pub audio_duration: f64,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Translated transcription with TTS audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTranslationResult {
    /// Original transcription
    pub transcription: TranscriptionResult,
    /// Translated text
    pub translated_text: String,
    /// Target language
    pub target_language: String,
    /// TTS audio (base64 encoded, WAV format)
    pub tts_audio: Option<String>,
    /// Total pipeline latency in milliseconds
    pub total_latency_ms: u64,
}

/// WebSocket message from Rust bot to voice inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VoiceInferenceRequest {
    /// Audio data for processing
    Audio {
        /// Guild ID
        guild_id: String,
        /// Channel ID
        channel_id: String,
        /// User ID
        user_id: String,
        /// Username
        username: String,
        /// Audio samples as base64 (i16 PCM, 48kHz, mono)
        audio_base64: String,
        /// Sample rate
        sample_rate: u32,
        /// Target language for translation
        target_language: String,
        /// Whether to generate TTS audio
        generate_tts: bool,
    },
    /// Ping to keep connection alive
    Ping,
    /// Configuration update
    Configure {
        /// Model settings
        stt_model: Option<String>,
        tts_model: Option<String>,
    },
}

/// WebSocket message from voice inference to Rust bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VoiceInferenceResponse {
    /// Transcription and translation result
    Result {
        /// Guild ID
        guild_id: String,
        /// Channel ID
        channel_id: String,
        /// User ID
        user_id: String,
        /// Username
        username: String,
        /// Original transcription
        original_text: String,
        /// Translated text
        translated_text: String,
        /// Source language
        source_language: String,
        /// Target language
        target_language: String,
        /// TTS audio as base64 (if requested)
        tts_audio: Option<String>,
        /// Pipeline latency in milliseconds
        latency_ms: u64,
    },
    /// Pong response
    Pong,
    /// Error from inference service
    Error {
        /// Error message
        message: String,
        /// Error code
        code: Option<String>,
    },
    /// Service ready notification
    Ready {
        /// Available STT models
        stt_models: Vec<String>,
        /// Available TTS models
        tts_models: Vec<String>,
    },
}

/// Voice channel state.
#[derive(Debug, Clone)]
pub struct VoiceChannelState {
    /// Guild ID
    pub guild_id: u64,
    /// Voice channel ID
    pub channel_id: u64,
    /// Whether translation is enabled
    pub translation_enabled: bool,
    /// Target language for translations
    pub target_language: String,
    /// Whether TTS playback is enabled
    pub tts_enabled: bool,
    /// Active speakers (SSRC -> user mapping)
    pub speakers: std::collections::HashMap<Ssrc, SpeakerInfo>,
}

impl Default for VoiceChannelState {
    fn default() -> Self {
        Self {
            guild_id: 0,
            channel_id: 0,
            translation_enabled: true,
            target_language: "en".to_string(),
            tts_enabled: false,
            speakers: std::collections::HashMap::new(),
        }
    }
}
