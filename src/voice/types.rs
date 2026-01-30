//! Shared types for voice translation pipeline.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
/// Config (target language, TTS settings) is passed separately when sending to inference.
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
        /// Audio hash for cache correlation (Python must echo this back)
        audio_hash: u64,
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
        /// Audio hash echoed back for cache correlation
        audio_hash: u64,
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
    /// Target language for translations (Arc so cloning is cheap)
    pub target_language: Arc<str>,
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
            target_language: Arc::from("en"),
            tts_enabled: false,
            speakers: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_segment_duration() {
        let start = Instant::now();
        let end = start + Duration::from_millis(1500);

        let segment = AudioSegment {
            user_id: 123,
            username: "Test".to_string(),
            guild_id: 456,
            channel_id: 789,
            samples: vec![1, 2, 3],
            start_time: start,
            end_time: end,
        };

        let duration = segment.duration();
        assert_eq!(duration.as_millis(), 1500);
    }

    #[test]
    fn test_audio_segment_samples_f32() {
        let segment = AudioSegment {
            user_id: 1,
            username: "Test".to_string(),
            guild_id: 2,
            channel_id: 3,
            samples: vec![0, 16384, -16384, 32767, -32768],
            start_time: Instant::now(),
            end_time: Instant::now(),
        };

        let f32_samples = segment.samples_f32();

        assert!((f32_samples[0] - 0.0).abs() < 0.001);
        assert!((f32_samples[1] - 0.5).abs() < 0.01);
        assert!((f32_samples[2] + 0.5).abs() < 0.01);
        assert!((f32_samples[3] - 1.0).abs() < 0.01);
        assert!((f32_samples[4] + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_audio_segment_samples_bytes() {
        let segment = AudioSegment {
            user_id: 1,
            username: "Test".to_string(),
            guild_id: 2,
            channel_id: 3,
            samples: vec![256, 512],
            start_time: Instant::now(),
            end_time: Instant::now(),
        };

        let bytes = segment.samples_bytes();
        assert_eq!(bytes.len(), 4); // 2 samples * 2 bytes

        // Verify little-endian encoding
        let reconstructed = vec![
            i16::from_le_bytes([bytes[0], bytes[1]]),
            i16::from_le_bytes([bytes[2], bytes[3]]),
        ];
        assert_eq!(reconstructed, vec![256, 512]);
    }

    #[test]
    fn test_voice_inference_request_audio() {
        let request = VoiceInferenceRequest::Audio {
            guild_id: "123".to_string(),
            channel_id: "456".to_string(),
            user_id: "789".to_string(),
            username: "TestUser".to_string(),
            audio_base64: "dGVzdA==".to_string(),
            sample_rate: 48000,
            target_language: "es".to_string(),
            generate_tts: true,
            audio_hash: 12345,
        };

        match request {
            VoiceInferenceRequest::Audio { audio_hash, .. } => {
                assert_eq!(audio_hash, 12345);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_voice_inference_response_result() {
        let response = VoiceInferenceResponse::Result {
            guild_id: "111".to_string(),
            channel_id: "222".to_string(),
            user_id: "333".to_string(),
            username: "User".to_string(),
            original_text: "hello".to_string(),
            translated_text: "hola".to_string(),
            source_language: "en".to_string(),
            target_language: "es".to_string(),
            tts_audio: None,
            latency_ms: 150,
            audio_hash: 67890,
        };

        match response {
            VoiceInferenceResponse::Result {
                audio_hash,
                latency_ms,
                ..
            } => {
                assert_eq!(audio_hash, 67890);
                assert_eq!(latency_ms, 150);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_voice_channel_state_default() {
        let state = VoiceChannelState::default();

        assert_eq!(state.guild_id, 0);
        assert_eq!(state.channel_id, 0);
        assert_eq!(state.translation_enabled, true);
        assert_eq!(state.target_language.as_ref(), "en");
        assert_eq!(state.tts_enabled, false);
        assert_eq!(state.speakers.len(), 0);
    }

    #[test]
    fn test_audio_packet_duration() {
        let packet = AudioPacket {
            ssrc: 12345,
            user_id: Some(678),
            username: Some("Test".to_string()),
            samples: vec![0; 960], // 20ms at 48kHz
            timestamp: Instant::now(),
            sequence: 0,
        };

        let duration = packet.duration();
        assert_eq!(duration.as_millis(), 20);
    }

    #[test]
    fn test_discord_sample_rate_constant() {
        assert_eq!(DISCORD_SAMPLE_RATE, 48000);
    }

    #[test]
    fn test_samples_per_frame_constant() {
        // 48000 Hz * 20ms / 1000 = 960 samples
        assert_eq!(SAMPLES_PER_FRAME, 960);
    }
}

