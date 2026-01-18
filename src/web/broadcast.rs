use crate::translation::TranslationResult;
use crate::voice::VoiceInferenceResponse;
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::broadcast;

/// Message sent to web clients via WebSocket
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WebMessage {
    /// Text channel translation
    #[serde(rename = "translation")]
    Translation(TextTranslationMessage),
    /// Voice channel transcription/translation
    #[serde(rename = "voice_transcription")]
    VoiceTranscription(VoiceTranscriptionMessage),
}

/// Text translation message (from text channels)
#[derive(Debug, Clone, Serialize)]
pub struct TextTranslationMessage {
    pub channel_id: String,
    pub author_name: String,
    pub author_id: String,
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub timestamp: i64,
}

/// Voice transcription message (from voice channels)
#[derive(Debug, Clone, Serialize)]
pub struct VoiceTranscriptionMessage {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub latency_ms: u64,
    pub timestamp: i64,
    /// Base64-encoded TTS audio (WAV format, 24kHz) if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_audio: Option<String>,
}

impl WebMessage {
    pub fn from_translation(
        channel_id: &str,
        author_name: &str,
        author_id: &str,
        translation: &TranslationResult,
    ) -> Self {
        Self::Translation(TextTranslationMessage {
            channel_id: channel_id.to_string(),
            author_name: author_name.to_string(),
            author_id: author_id.to_string(),
            original_text: translation.original_text.clone(),
            translated_text: translation.translated_text.clone(),
            source_lang: translation.source_lang.clone(),
            target_lang: translation.target_lang.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    pub fn from_voice_transcription(response: &VoiceInferenceResponse) -> Option<Self> {
        match response {
            VoiceInferenceResponse::Result {
                guild_id,
                channel_id,
                user_id,
                username,
                original_text,
                translated_text,
                source_language,
                target_language,
                tts_audio,
                latency_ms,
            } => {
                // Skip empty transcriptions
                if original_text.is_empty() {
                    return None;
                }

                Some(Self::VoiceTranscription(VoiceTranscriptionMessage {
                    guild_id: guild_id.clone(),
                    channel_id: channel_id.clone(),
                    user_id: user_id.clone(),
                    username: username.clone(),
                    original_text: original_text.clone(),
                    translated_text: translated_text.clone(),
                    source_lang: source_language.clone(),
                    target_lang: target_language.clone(),
                    latency_ms: *latency_ms,
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    tts_audio: tts_audio.clone(),
                }))
            }
            _ => None,
        }
    }
}

/// Manages broadcast channels for real-time web updates
pub struct BroadcastManager {
    /// Global broadcast channel for all translations
    global_tx: broadcast::Sender<WebMessage>,
    /// Per-channel broadcast channels
    channel_txs: DashMap<String, broadcast::Sender<WebMessage>>,
}

impl std::fmt::Debug for BroadcastManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BroadcastManager")
            .field("global_subscribers", &self.global_tx.receiver_count())
            .field("channel_count", &self.channel_txs.len())
            .finish()
    }
}

impl BroadcastManager {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(1000);
        Self {
            global_tx,
            channel_txs: DashMap::new(),
        }
    }

    /// Subscribe to all translations
    pub fn subscribe_global(&self) -> broadcast::Receiver<WebMessage> {
        self.global_tx.subscribe()
    }

    /// Subscribe to a specific channel's translations
    pub fn subscribe_channel(&self, channel_id: &str) -> broadcast::Receiver<WebMessage> {
        let tx = self.channel_txs.entry(channel_id.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            tx
        });
        tx.subscribe()
    }

    /// Send a translation to subscribers
    pub fn send_translation(
        &self,
        channel_id: &str,
        author_name: &str,
        author_id: &str,
        translation: &TranslationResult,
    ) {
        let msg = WebMessage::from_translation(channel_id, author_name, author_id, translation);

        // Send to global subscribers
        let _ = self.global_tx.send(msg.clone());

        // Send to channel-specific subscribers
        if let Some(tx) = self.channel_txs.get(channel_id) {
            let _ = tx.send(msg);
        }
    }

    /// Subscribe to a specific voice channel's transcriptions.
    ///
    /// Uses guild_id:channel_id as the key for voice channels.
    pub fn subscribe_voice_channel(
        &self,
        guild_id: &str,
        channel_id: &str,
    ) -> broadcast::Receiver<WebMessage> {
        let key = format!("voice:{}:{}", guild_id, channel_id);
        let tx = self.channel_txs.entry(key).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            tx
        });
        tx.subscribe()
    }

    /// Send a voice transcription to subscribers
    pub fn send_voice_transcription(&self, response: &VoiceInferenceResponse) {
        if let Some(msg) = WebMessage::from_voice_transcription(response) {
            // Send to global subscribers
            let _ = self.global_tx.send(msg.clone());

            // Send to voice channel-specific subscribers
            if let VoiceInferenceResponse::Result {
                guild_id,
                channel_id,
                ..
            } = response
            {
                let key = format!("voice:{}:{}", guild_id, channel_id);
                // Create channel if subscribers exist, otherwise just try to send
                if let Some(tx) = self.channel_txs.get(&key) {
                    let _ = tx.send(msg);
                }
            }
        }
    }

    /// Get number of global subscribers
    pub fn global_subscriber_count(&self) -> usize {
        self.global_tx.receiver_count()
    }

    /// Clean up unused channel senders
    pub fn cleanup_empty_channels(&self) {
        self.channel_txs.retain(|_, tx| tx.receiver_count() > 0);
    }
}

impl Default for BroadcastManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_manager_creation() {
        let manager = BroadcastManager::new();
        assert_eq!(manager.global_subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_global_subscription() {
        let manager = BroadcastManager::new();
        let mut rx = manager.subscribe_global();

        let translation = TranslationResult {
            original_text: "Hello".to_string(),
            translated_text: "Hola".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
            cached: false,
        };

        manager.send_translation("123", "TestUser", "456", &translation);

        let msg = rx.try_recv().unwrap();
        match msg {
            WebMessage::Translation(t) => {
                assert_eq!(t.translated_text, "Hola");
                assert_eq!(t.channel_id, "123");
            }
            _ => panic!("Expected Translation message"),
        }
    }
}
