//! Per-user audio ring buffers with voice activity detection.

use super::types::{AudioPacket, AudioSegment, Ssrc, DISCORD_SAMPLE_RATE, SAMPLES_PER_FRAME};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Minimum duration of speech to trigger transcription (ms).
const MIN_SPEECH_DURATION_MS: u64 = 500;

/// Maximum duration of speech before forced flush (seconds).
const MAX_SPEECH_DURATION_SECS: u64 = 30;

/// Silence duration to consider speech ended (ms).
const SILENCE_TIMEOUT_MS: u64 = 800;

/// Simple energy-based VAD threshold.
const VAD_ENERGY_THRESHOLD: f32 = 0.01;

/// Per-user audio buffer.
#[derive(Debug)]
struct UserBuffer {
    /// Discord user ID
    user_id: u64,
    /// Username
    username: String,
    /// Guild ID
    guild_id: u64,
    /// Voice channel ID
    channel_id: u64,
    /// Accumulated audio samples
    samples: Vec<i16>,
    /// When this utterance started
    speech_start: Option<Instant>,
    /// Last time we received audio
    last_audio_time: Instant,
    /// Is user currently speaking?
    is_speaking: bool,
}

impl UserBuffer {
    fn new(user_id: u64, username: String, guild_id: u64, channel_id: u64) -> Self {
        Self {
            user_id,
            username,
            guild_id,
            channel_id,
            samples: Vec::with_capacity(SAMPLES_PER_FRAME * 50), // ~1 second initial capacity
            speech_start: None,
            last_audio_time: Instant::now(),
            is_speaking: false,
        }
    }

    /// Add audio samples to buffer.
    fn push_audio(&mut self, samples: &[i16]) {
        let now = Instant::now();
        let has_speech = detect_speech(samples);

        if has_speech {
            if !self.is_speaking {
                // Speech started
                self.is_speaking = true;
                self.speech_start = Some(now);
                trace!(user_id = self.user_id, "Speech started");
            }
            self.samples.extend_from_slice(samples);
            self.last_audio_time = now;
        } else if self.is_speaking {
            // Still include some silence for natural speech boundaries
            self.samples.extend_from_slice(samples);
        }
    }

    /// Check if we should flush this buffer.
    fn should_flush(&self) -> bool {
        if !self.is_speaking || self.samples.is_empty() {
            return false;
        }

        let now = Instant::now();
        let speech_start = self.speech_start.unwrap_or(now);
        let speech_duration = now.duration_since(speech_start);
        let silence_duration = now.duration_since(self.last_audio_time);

        // Flush if silence timeout reached
        if silence_duration >= Duration::from_millis(SILENCE_TIMEOUT_MS) {
            let total_duration = self.samples.len() as f64 / DISCORD_SAMPLE_RATE as f64;
            if total_duration >= MIN_SPEECH_DURATION_MS as f64 / 1000.0 {
                return true;
            }
        }

        // Flush if max duration reached
        if speech_duration >= Duration::from_secs(MAX_SPEECH_DURATION_SECS) {
            return true;
        }

        false
    }

    /// Flush buffer and return audio segment.
    fn flush(&mut self) -> Option<AudioSegment> {
        if self.samples.is_empty() {
            return None;
        }

        let segment = AudioSegment {
            user_id: self.user_id,
            username: self.username.clone(),
            guild_id: self.guild_id,
            channel_id: self.channel_id,
            samples: std::mem::take(&mut self.samples),
            start_time: self.speech_start.unwrap_or_else(Instant::now),
            end_time: Instant::now(),
        };

        self.speech_start = None;
        self.is_speaking = false;
        self.samples = Vec::with_capacity(SAMPLES_PER_FRAME * 50);

        debug!(
            user_id = self.user_id,
            duration_ms = segment.duration().as_millis(),
            samples = segment.samples.len(),
            "Flushed audio buffer"
        );

        Some(segment)
    }

    /// Force flush due to timeout.
    fn force_flush(&mut self) -> Option<AudioSegment> {
        if self.samples.is_empty() {
            return None;
        }
        self.flush()
    }
}

/// Simple energy-based voice activity detection.
fn detect_speech(samples: &[i16]) -> bool {
    if samples.is_empty() {
        return false;
    }

    // Calculate RMS energy
    let sum_squares: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (sum_squares / samples.len() as f64).sqrt() / 32768.0;

    rms > VAD_ENERGY_THRESHOLD as f64
}

/// Manages audio buffers for all users in a voice channel.
pub struct AudioBufferManager {
    /// SSRC -> user buffer mapping
    buffers: Arc<RwLock<HashMap<Ssrc, UserBuffer>>>,
    /// SSRC -> User ID mapping (populated from SpeakingStateUpdate)
    ssrc_map: Arc<RwLock<HashMap<Ssrc, (u64, String)>>>,
    /// Guild ID
    guild_id: u64,
    /// Channel ID
    channel_id: u64,
}

impl AudioBufferManager {
    /// Create a new buffer manager for a voice channel.
    pub fn new(guild_id: u64, channel_id: u64) -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            ssrc_map: Arc::new(RwLock::new(HashMap::new())),
            guild_id,
            channel_id,
        }
    }

    /// Register SSRC to user ID mapping.
    pub async fn register_speaker(&self, ssrc: Ssrc, user_id: u64, username: String) {
        let mut ssrc_map = self.ssrc_map.write().await;
        ssrc_map.insert(ssrc, (user_id, username.clone()));

        let mut buffers = self.buffers.write().await;
        buffers
            .entry(ssrc)
            .or_insert_with(|| UserBuffer::new(user_id, username, self.guild_id, self.channel_id));

        debug!(ssrc, user_id, "Registered speaker");
    }

    /// Remove speaker from tracking.
    pub async fn unregister_speaker(&self, ssrc: Ssrc) -> Option<AudioSegment> {
        let mut ssrc_map = self.ssrc_map.write().await;
        ssrc_map.remove(&ssrc);

        let mut buffers = self.buffers.write().await;
        if let Some(mut buffer) = buffers.remove(&ssrc) {
            return buffer.force_flush();
        }
        None
    }

    /// Process incoming audio packet.
    pub async fn push_audio(&self, packet: AudioPacket) -> Option<AudioSegment> {
        let ssrc_map = self.ssrc_map.read().await;
        let (user_id, username) = ssrc_map.get(&packet.ssrc)?.clone();
        drop(ssrc_map);

        let mut buffers = self.buffers.write().await;
        let buffer = buffers
            .entry(packet.ssrc)
            .or_insert_with(|| UserBuffer::new(user_id, username, self.guild_id, self.channel_id));

        buffer.push_audio(&packet.samples);

        if buffer.should_flush() {
            return buffer.flush();
        }

        None
    }

    /// Check all buffers for timeout and flush if needed.
    pub async fn check_timeouts(&self) -> Vec<AudioSegment> {
        let mut segments = Vec::new();
        let mut buffers = self.buffers.write().await;

        for buffer in buffers.values_mut() {
            if buffer.should_flush() {
                if let Some(segment) = buffer.flush() {
                    segments.push(segment);
                }
            }
        }

        segments
    }

    /// Flush all buffers (e.g., when leaving channel).
    pub async fn flush_all(&self) -> Vec<AudioSegment> {
        let mut segments = Vec::new();
        let mut buffers = self.buffers.write().await;

        for buffer in buffers.values_mut() {
            if let Some(segment) = buffer.force_flush() {
                segments.push(segment);
            }
        }

        segments
    }

    /// Get number of active speakers.
    pub async fn speaker_count(&self) -> usize {
        self.ssrc_map.read().await.len()
    }
}

impl std::fmt::Debug for AudioBufferManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioBufferManager")
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_speech_silence() {
        let silence = vec![0i16; 960];
        assert!(!detect_speech(&silence));
    }

    #[test]
    fn test_detect_speech_audio() {
        // Generate a simple sine wave
        let samples: Vec<i16> = (0..960)
            .map(|i| ((i as f32 * 0.1).sin() * 10000.0) as i16)
            .collect();
        assert!(detect_speech(&samples));
    }

    #[tokio::test]
    async fn test_buffer_manager() {
        let manager = AudioBufferManager::new(123, 456);
        manager
            .register_speaker(1, 789, "TestUser".to_string())
            .await;
        assert_eq!(manager.speaker_count().await, 1);
    }
}
