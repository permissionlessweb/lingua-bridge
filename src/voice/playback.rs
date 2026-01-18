//! TTS audio playback to Discord voice channel.

use super::types::VoiceInferenceResponse;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use songbird::{tracks::TrackHandle, Call};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};

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
async fn play_tts_audio(
    call: &Arc<tokio::sync::Mutex<Call>>,
    item: &TTSPlaybackItem,
) -> Result<(), PlaybackError> {
    info!(user = item.username, text = item.text, "Playing TTS audio");

    // Convert i16 samples to f32 for songbird
    let samples_f32: Vec<f32> = item.audio.iter().map(|&s| s as f32 / 32768.0).collect();

    // Create a simple PCM input
    // Songbird expects stereo, so we'll duplicate mono to stereo
    let stereo_samples: Vec<f32> = samples_f32.iter().flat_map(|&s| [s, s]).collect();

    // For now, we'll use a simple approach
    // In production, you'd want proper audio streaming
    let _call = call.lock().await;

    // Songbird's Input system is complex; this is a simplified placeholder
    // The actual implementation would use Input::new() with proper audio source
    // TODO: Implement proper audio streaming with songbird's Input API

    debug!(
        samples = stereo_samples.len(),
        sample_rate = item.sample_rate,
        "TTS playback complete"
    );

    Ok(())
}

/// Playback errors.
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("No audio data")]
    NoAudio,

    #[error("Invalid audio format")]
    InvalidFormat,

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
}
