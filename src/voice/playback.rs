//! TTS audio playback to Discord voice channel.

use super::types::VoiceInferenceResponse;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use songbird::{
    input::{Input, RawAdapter},
    tracks::TrackHandle,
    Call,
};
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

/// Playback manager for TTS audio.
pub struct PlaybackManager {
    /// Queue of pending TTS audio to play
    queue: Arc<RwLock<Vec<TTSPlaybackItem>>>,
    /// Whether playback is currently active
    playing: Arc<RwLock<bool>>,
    /// Current track handle if playing
    _current_track: Arc<RwLock<Option<TrackHandle>>>,
}

/// Item in the TTS playback queue.
#[derive(Debug, Clone)]
pub struct TTSPlaybackItem {
    /// User ID who triggered this TTS
    pub user_id: u64,
    /// Username
    pub username: String,
    /// The translated text being spoken
    pub text: String,
    /// Audio data (PCM samples)
    pub audio: Vec<i16>,
    /// Sample rate
    pub sample_rate: u32,
}

impl PlaybackManager {
    /// Create a new playback manager.
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(Vec::new())),
            playing: Arc::new(RwLock::new(false)),
            _current_track: Arc::new(RwLock::new(None)),
        }
    }

    /// Queue TTS audio for playback.
    pub async fn queue_tts(&self, item: TTSPlaybackItem) {
        let mut queue = self.queue.write().await;
        queue.push(item);
        debug!(queue_len = queue.len(), "Queued TTS for playback");
    }

    /// Get the next item from the queue.
    pub async fn next(&self) -> Option<TTSPlaybackItem> {
        let mut queue = self.queue.write().await;
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    }

    /// Check if currently playing.
    pub async fn is_playing(&self) -> bool {
        *self.playing.read().await
    }

    /// Set playing state.
    pub async fn set_playing(&self, playing: bool) {
        *self.playing.write().await = playing;
    }

    /// Get queue length.
    pub async fn queue_len(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Clear the queue.
    pub async fn clear(&self) {
        self.queue.write().await.clear();
    }
}

impl Default for PlaybackManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse TTS audio from inference response.
pub fn parse_tts_audio(response: &VoiceInferenceResponse) -> Option<TTSPlaybackItem> {
    match response {
        VoiceInferenceResponse::Result {
            user_id,
            username,
            translated_text,
            tts_audio,
            ..
        } => {
            let audio_base64 = tts_audio.as_ref()?;

            // Decode base64 audio
            let audio_bytes = BASE64.decode(audio_base64).ok()?;

            // Convert bytes to i16 samples (assuming little-endian PCM)
            let samples: Vec<i16> = audio_bytes
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            if samples.is_empty() {
                return None;
            }

            Some(TTSPlaybackItem {
                user_id: user_id.parse().unwrap_or(0),
                username: username.clone(),
                text: translated_text.clone(),
                audio: samples,
                sample_rate: 24000, // CosyVoice typically outputs 24kHz
            })
        }
        _ => None,
    }
}

/// Start the TTS playback loop for a voice channel.
pub async fn run_playback_loop(
    call: Arc<tokio::sync::Mutex<Call>>,
    playback_manager: Arc<PlaybackManager>,
    mut result_rx: broadcast::Receiver<VoiceInferenceResponse>,
) {
    info!("Starting TTS playback loop");

    loop {
        tokio::select! {
            Ok(response) = result_rx.recv() => {
                if let Some(item) = parse_tts_audio(&response) {
                    debug!(
                        user = item.username,
                        text_len = item.text.len(),
                        audio_samples = item.audio.len(),
                        "Received TTS audio"
                    );
                    playback_manager.queue_tts(item).await;
                }
            }

            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Check if we should start playing
                if !playback_manager.is_playing().await {
                    if let Some(item) = playback_manager.next().await {
                        playback_manager.set_playing(true).await;

                        // Play the TTS audio
                        if let Err(e) = play_tts_audio(&call, &item).await {
                            error!(error = %e, "Failed to play TTS audio");
                        }

                        playback_manager.set_playing(false).await;
                    }
                }
            }
        }
    }
}

/// Play TTS audio through the voice connection.
///
/// # Audio Pipeline
///
/// This function handles the complete audio pipeline from TTS output to Discord playback:
///
/// 1. **Input**: Mono PCM i16 samples at variable sample rate (typically 24kHz from CosyVoice)
/// 2. **Resampling**: Convert to 48kHz using linear interpolation (Discord requirement)
/// 3. **Channel conversion**: Duplicate mono to stereo (Discord requirement)
/// 4. **Encoding**: Songbird handles Opus encoding automatically
/// 5. **Playback**: Stream to Discord voice channel
///
/// # Discord Audio Requirements
///
/// - Sample rate: 48kHz (hardcoded in Discord's Opus configuration)
/// - Channels: Stereo (2 channels)
/// - Format: 16-bit signed PCM (Songbird converts to Opus)
/// - Frame size: 20ms (960 samples per channel at 48kHz)
///
/// # Performance Notes
///
/// - Linear interpolation resampling is fast but lower quality than sinc/polyphase
/// - For better quality, consider using the `rubato` crate for professional resampling
/// - Current implementation loads entire audio into memory (fine for short TTS clips)
/// - For longer audio, consider streaming/chunked processing
async fn play_tts_audio(
    call: &Arc<tokio::sync::Mutex<Call>>,
    item: &TTSPlaybackItem,
) -> Result<(), PlaybackError> {
    info!(
        user = item.username,
        text = item.text,
        samples = item.audio.len(),
        sample_rate = item.sample_rate,
        "Playing TTS audio"
    );

    if item.audio.is_empty() {
        return Err(PlaybackError::NoAudio);
    }

    // Discord voice requires 48kHz stereo PCM (Songbird handles Opus encoding)
    const DISCORD_SAMPLE_RATE: u32 = 48000;

    // Prepare audio data: resample to 48kHz and convert to stereo
    let stereo_48k = prepare_audio_for_discord(&item.audio, item.sample_rate)?;

    // Convert i16 stereo samples to bytes (little-endian PCM)
    let audio_bytes: Vec<u8> = stereo_48k
        .iter()
        .flat_map(|&sample| sample.to_le_bytes())
        .collect();

    debug!(
        original_samples = item.audio.len(),
        resampled_samples = stereo_48k.len(),
        bytes = audio_bytes.len(),
        "Audio prepared for playback"
    );

    // Create an in-memory cursor for the audio data
    let cursor = Cursor::new(audio_bytes);

    // Create Songbird input from raw PCM data
    // RawAdapter: stereo 16-bit signed PCM at 48kHz
    let input = Input::from(RawAdapter::new(
        cursor,
        DISCORD_SAMPLE_RATE,
        2, // stereo channels
    ));

    // Play the audio through the voice connection
    let mut handler = call.lock().await;
    let track_handle = handler.play_input(input);

    // Wait for the track to finish playing
    let duration_secs = stereo_48k.len() as f64 / (DISCORD_SAMPLE_RATE as f64 * 2.0);
    debug!(duration_secs = duration_secs, "Waiting for playback to complete");

    // Release the lock while waiting
    drop(handler);

    // Wait for playback to complete (with a small buffer)
    let wait_duration = std::time::Duration::from_secs_f64(duration_secs + 0.5);
    tokio::time::sleep(wait_duration).await;

    // Check if track finished successfully
    if let Err(e) = track_handle.get_info().await {
        warn!(error = ?e, "Failed to get track info");
    }

    debug!("TTS playback complete");
    Ok(())
}

/// Prepare audio for Discord: resample to 48kHz and convert to stereo.
fn prepare_audio_for_discord(
    mono_samples: &[i16],
    source_sample_rate: u32,
) -> Result<Vec<i16>, PlaybackError> {
    const DISCORD_SAMPLE_RATE: u32 = 48000;

    // First, resample to 48kHz if needed
    let resampled = if source_sample_rate != DISCORD_SAMPLE_RATE {
        resample_audio(mono_samples, source_sample_rate, DISCORD_SAMPLE_RATE)
    } else {
        mono_samples.to_vec()
    };

    // Convert mono to stereo by duplicating each sample
    let stereo: Vec<i16> = resampled.iter().flat_map(|&s| [s, s]).collect();

    Ok(stereo)
}

/// Simple linear interpolation resampling.
///
/// This uses basic linear interpolation which is fast but lower quality than
/// professional resampling algorithms (sinc, polyphase, etc.). For TTS playback
/// the quality is generally acceptable.
///
/// # Algorithm
///
/// For each output sample position:
/// 1. Calculate corresponding position in input samples
/// 2. If between two samples, linearly interpolate
/// 3. If at last sample, use that sample directly
///
/// # Production Improvements
///
/// For better quality, consider using:
/// - `rubato` crate: High-quality resampling with various algorithms
/// - `samplerate` crate: libsamplerate bindings (sinc interpolation)
/// - `dasp` crate: Digital audio signal processing primitives
///
/// # Example
///
/// ```ignore
/// // Upsample 24kHz to 48kHz (2x)
/// let input = vec![100i16, 200, 300];
/// let output = resample_audio(&input, 24000, 48000);
/// // output will have ~6 samples with interpolated values
/// ```
fn resample_audio(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    // No-op if rates match
    if from_rate == to_rate {
        return samples.to_vec();
    }

    // Handle empty input
    if samples.is_empty() {
        return Vec::new();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (samples.len() as f64 * ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos as usize;

        if src_idx + 1 < samples.len() {
            // Linear interpolation between two samples
            let frac = src_pos - src_idx as f64;
            let sample1 = samples[src_idx] as f64;
            let sample2 = samples[src_idx + 1] as f64;
            let interpolated = sample1 + (sample2 - sample1) * frac;

            // Clamp to i16 range to prevent overflow
            let clamped = interpolated.clamp(i16::MIN as f64, i16::MAX as f64);
            output.push(clamped as i16);
        } else if src_idx < samples.len() {
            // Last sample, no interpolation needed
            output.push(samples[src_idx]);
        } else {
            // Shouldn't happen, but handle gracefully
            if let Some(&last) = samples.last() {
                output.push(last);
            }
        }
    }

    output
}

/// Playback errors.
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("No audio data")]
    NoAudio,

    #[error("Invalid audio format: {0}")]
    InvalidFormat(String),

    #[error("Resampling failed: unsupported sample rate conversion {from}Hz -> {to}Hz")]
    UnsupportedSampleRate { from: u32, to: u32 },

    #[error("Playback failed: {0}")]
    Failed(String),
}

impl std::fmt::Debug for PlaybackManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaybackManager").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_playback_manager() {
        let manager = PlaybackManager::new();
        assert_eq!(manager.queue_len().await, 0);
        assert!(!manager.is_playing().await);
    }

    #[tokio::test]
    async fn test_queue_tts() {
        let manager = PlaybackManager::new();

        let item = TTSPlaybackItem {
            user_id: 123,
            username: "Test".to_string(),
            text: "Hello".to_string(),
            audio: vec![0i16; 1000],
            sample_rate: 24000,
        };

        manager.queue_tts(item).await;
        assert_eq!(manager.queue_len().await, 1);

        let next = manager.next().await;
        assert!(next.is_some());
        assert_eq!(manager.queue_len().await, 0);
    }

    #[test]
    fn test_resample_audio() {
        // Test 2x upsampling (24kHz -> 48kHz)
        let input = vec![100i16, 200, 300, 400];
        let output = resample_audio(&input, 24000, 48000);

        // Should have ~2x samples
        assert_eq!(output.len(), 8);

        // First sample should be the same
        assert_eq!(output[0], 100);

        // Check interpolation works
        assert!(output[1] > 100 && output[1] < 200);
    }

    #[test]
    fn test_resample_audio_same_rate() {
        // Test no-op when rates are the same
        let input = vec![100i16, 200, 300];
        let output = resample_audio(&input, 48000, 48000);

        assert_eq!(input, output);
    }

    #[test]
    fn test_prepare_audio_for_discord() {
        let mono = vec![100i16, 200, 300];
        let result = prepare_audio_for_discord(&mono, 48000).unwrap();

        // Mono -> Stereo: each sample duplicated
        assert_eq!(result.len(), 6);
        assert_eq!(result[0], 100);
        assert_eq!(result[1], 100); // duplicated
        assert_eq!(result[2], 200);
        assert_eq!(result[3], 200); // duplicated
    }

    #[test]
    fn test_prepare_audio_with_resampling() {
        let mono = vec![100i16, 200, 300, 400];
        let result = prepare_audio_for_discord(&mono, 24000).unwrap();

        // 24kHz -> 48kHz = 2x samples, then stereo = 2x again = 4x total
        // 4 samples * 2 (resample) * 2 (stereo) = 16 samples
        assert_eq!(result.len(), 16);
    }
}
