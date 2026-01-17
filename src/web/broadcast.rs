use crate::translation::TranslationResult;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Message sent to web clients via WebSocket
#[derive(Debug, Clone, Serialize)]
pub struct WebMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel_id: String,
    pub author_name: String,
    pub author_id: String,
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub timestamp: i64,
}

impl WebMessage {
    pub fn from_translation(
        channel_id: &str,
        author_name: &str,
        author_id: &str,
        translation: &TranslationResult,
    ) -> Self {
        Self {
            msg_type: "translation".to_string(),
            channel_id: channel_id.to_string(),
            author_name: author_name.to_string(),
            author_id: author_id.to_string(),
            original_text: translation.original_text.clone(),
            translated_text: translation.translated_text.clone(),
            source_lang: translation.source_lang.clone(),
            target_lang: translation.target_lang.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
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
        assert_eq!(msg.translated_text, "Hola");
        assert_eq!(msg.channel_id, "123");
    }
}
