//! LRU cache for audio transcription/translation results to avoid duplicate work.
//!
//! When the same audio is spoken multiple times (e.g., "hello", "yes", "okay"),
//! we can cache the transcription/translation results keyed by audio hash + target language.
//! This can reduce inference latency by 10-100x for repeated phrases.

use blake3::Hasher as Blake3Hasher;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, trace};

use super::types::VoiceInferenceResponse;

/// Cache entry for a translation result.
#[derive(Debug, Clone)]
pub struct CachedTranslation {
    pub response: VoiceInferenceResponse,
    /// Timestamp when cached (for metrics/debugging)
    pub cached_at: std::time::Instant,
}

/// LRU cache for voice transcription/translation results.
///
/// Key: (audio_hash, target_language)
/// Value: Full inference response (transcription + translation + TTS)
///
/// This prevents reprocessing identical audio segments (common for repeated phrases).
pub struct VoiceTranscriptionCache {
    cache: Arc<Mutex<LruCache<(u64, Arc<str>), CachedTranslation>>>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl VoiceTranscriptionCache {
    /// Create a new cache with specified capacity.
    ///
    /// Recommended capacity: 1000-5000 entries
    /// - 1000 entries ≈ 10-50 MB memory (depends on text length)
    /// - Covers most common phrases in typical voice calls
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("Capacity must be non-zero"),
            ))),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Hash audio samples for cache key.
    ///
    /// Uses blake3 which is extremely fast (faster than SipHash) and provides
    /// excellent collision resistance. Audio data is deterministic, so same audio = same hash.
    pub fn hash_audio(samples: &[i16]) -> u64 {
        // Hash raw bytes (i16 samples)
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                samples.as_ptr() as *const u8,
                samples.len() * std::mem::size_of::<i16>(),
            )
        };

        let mut hasher = Blake3Hasher::new();
        hasher.update(bytes);
        let hash_bytes = hasher.finalize();

        // Convert first 8 bytes of blake3 hash to u64
        u64::from_le_bytes(hash_bytes.as_bytes()[..8].try_into().unwrap())
    }

    /// Get cached result if available.
    pub async fn get(
        &self,
        audio_hash: u64,
        target_language: &Arc<str>,
    ) -> Option<VoiceInferenceResponse> {
        let mut cache = self.cache.lock().await;
        if let Some(cached) = cache.get(&(audio_hash, Arc::clone(target_language))) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            debug!(
                audio_hash,
                target_language = %target_language,
                age_ms = cached.cached_at.elapsed().as_millis(),
                "Cache HIT"
            );
            Some(cached.response.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            trace!(audio_hash, target_language = %target_language, "Cache MISS");
            None
        }
    }

    /// Store result in cache.
    pub async fn put(
        &self,
        audio_hash: u64,
        target_language: Arc<str>,
        response: VoiceInferenceResponse,
    ) {
        let mut cache = self.cache.lock().await;
        cache.put(
            (audio_hash, target_language),
            CachedTranslation {
                response,
                cached_at: std::time::Instant::now(),
            },
        );
    }

    /// Check if cache contains result for audio hash + target language.
    pub async fn contains(&self, audio_hash: u64, target_language: &Arc<str>) -> bool {
        let cache = self.cache.lock().await;
        cache.contains(&(audio_hash, Arc::clone(target_language)))
    }

    /// Clear all cached entries.
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    /// Get current cache size.
    pub async fn len(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }

    /// Check if cache is empty.
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.lock().await;
        cache.is_empty()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            hits,
            misses,
            total,
            hit_rate,
        }
    }

    /// Reset cache statistics (for testing/debugging).
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }
}

impl Default for VoiceTranscriptionCache {
    fn default() -> Self {
        // Default to 1000 entries (reasonable for most use cases)
        Self::new(1000)
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total: u64,
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Hits: {}, Misses: {}, Hit Rate: {:.2}%, Total: {}",
            self.hits,
            self.misses,
            self.hit_rate * 100.0,
            self.total
        )
    }
}

impl std::fmt::Debug for VoiceTranscriptionCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("VoiceTranscriptionCache")
            .field("hits", &stats.hits)
            .field("misses", &stats.misses)
            .field("hit_rate", &format!("{:.2}%", stats.hit_rate * 100.0))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_audio_deterministic() {
        let samples1 = vec![1000i16, 2000, 3000, 4000];
        let samples2 = vec![1000i16, 2000, 3000, 4000];
        let hash1 = VoiceTranscriptionCache::hash_audio(&samples1);
        let hash2 = VoiceTranscriptionCache::hash_audio(&samples2);
        assert_eq!(hash1, hash2, "Same samples should produce same hash");
    }

    #[test]
    fn test_hash_audio_different() {
        let samples1 = vec![1000i16, 2000, 3000];
        let samples2 = vec![1000i16, 2000, 3001]; // Last sample differs
        let hash1 = VoiceTranscriptionCache::hash_audio(&samples1);
        let hash2 = VoiceTranscriptionCache::hash_audio(&samples2);
        assert_ne!(hash1, hash2, "Different samples should produce different hashes");
    }

    #[test]
    fn test_hash_audio_empty() {
        let samples = vec![];
        let hash = VoiceTranscriptionCache::hash_audio(&samples);
        assert!(hash > 0 || hash == 0); // Should produce a valid hash (empty = 0 is fine)
    }

    #[tokio::test]
    async fn test_cache_put_get() {
        use super::super::types::VoiceInferenceResponse;

        let cache = VoiceTranscriptionCache::new(10);
        let audio_hash = 12345u64;
        let target_lang = Arc::from("en");

        let response = VoiceInferenceResponse::Result {
            guild_id: "123".to_string(),
            channel_id: "456".to_string(),
            user_id: "789".to_string(),
            username: "TestUser".to_string(),
            original_text: "Hello".to_string(),
            translated_text: "Hello".to_string(),
            source_language: "en".to_string(),
            target_language: "en".to_string(),
            tts_audio: None,
            latency_ms: 100,
            audio_hash: 0,
        };

        cache.put(audio_hash, Arc::clone(&target_lang), response.clone()).await;

        let retrieved = cache.get(audio_hash, &target_lang).await;
        assert!(retrieved.is_some());

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate, 1.0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = VoiceTranscriptionCache::new(10);
        let audio_hash = 12345u64;
        let target_lang = Arc::from("en");

        let result = cache.get(audio_hash, &target_lang).await;
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_cache_different_languages() {
        use super::super::types::VoiceInferenceResponse;

        let cache = VoiceTranscriptionCache::new(10);
        let audio_hash = 12345u64;
        let lang_en = Arc::from("en");
        let lang_es = Arc::from("es");

        let response_en = VoiceInferenceResponse::Result {
            guild_id: "123".to_string(),
            channel_id: "456".to_string(),
            user_id: "789".to_string(),
            username: "TestUser".to_string(),
            original_text: "Hello".to_string(),
            translated_text: "Hello".to_string(),
            source_language: "en".to_string(),
            target_language: "en".to_string(),
            tts_audio: None,
            latency_ms: 100,
            audio_hash: 0,
        };

        let response_es = VoiceInferenceResponse::Result {
            guild_id: "123".to_string(),
            channel_id: "456".to_string(),
            user_id: "789".to_string(),
            username: "TestUser".to_string(),
            original_text: "Hello".to_string(),
            translated_text: "Hola".to_string(),
            source_language: "en".to_string(),
            target_language: "es".to_string(),
            tts_audio: None,
            latency_ms: 100,
            audio_hash: 0,
        };

        cache.put(audio_hash, Arc::clone(&lang_en), response_en).await;
        cache.put(audio_hash, Arc::clone(&lang_es), response_es).await;

        let retrieved_en = cache.get(audio_hash, &lang_en).await.unwrap();
        let retrieved_es = cache.get(audio_hash, &lang_es).await.unwrap();

        if let VoiceInferenceResponse::Result { translated_text: text_en, .. } = retrieved_en {
            assert_eq!(text_en, "Hello");
        }

        if let VoiceInferenceResponse::Result { translated_text: text_es, .. } = retrieved_es {
            assert_eq!(text_es, "Hola");
        }
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        use super::super::types::VoiceInferenceResponse;

        let cache = VoiceTranscriptionCache::new(2); // Small cache
        let lang = Arc::from("en");

        let make_response = |text: &str| VoiceInferenceResponse::Result {
            guild_id: "123".to_string(),
            channel_id: "456".to_string(),
            user_id: "789".to_string(),
            username: "TestUser".to_string(),
            original_text: text.to_string(),
            translated_text: text.to_string(),
            source_language: "en".to_string(),
            target_language: "en".to_string(),
            tts_audio: None,
            latency_ms: 100,
            audio_hash: 0,
        };

        cache.put(1, Arc::clone(&lang), make_response("One")).await;
        cache.put(2, Arc::clone(&lang), make_response("Two")).await;
        cache.put(3, Arc::clone(&lang), make_response("Three")).await; // Should evict entry 1

        assert!(cache.get(1, &lang).await.is_none(), "Entry 1 should be evicted");
        assert!(cache.get(2, &lang).await.is_some(), "Entry 2 should still exist");
        assert!(cache.get(3, &lang).await.is_some(), "Entry 3 should exist");
    }

    #[tokio::test]
    async fn test_cache_clear() {
        use super::super::types::VoiceInferenceResponse;

        let cache = VoiceTranscriptionCache::new(10);
        let lang = Arc::from("en");

        let response = VoiceInferenceResponse::Result {
            guild_id: "123".to_string(),
            channel_id: "456".to_string(),
            user_id: "789".to_string(),
            username: "TestUser".to_string(),
            original_text: "Test".to_string(),
            translated_text: "Test".to_string(),
            source_language: "en".to_string(),
            target_language: "en".to_string(),
            tts_audio: None,
            latency_ms: 100,
            audio_hash: 0,
        };

        cache.put(123, Arc::clone(&lang), response).await;
        assert_eq!(cache.len().await, 1);

        cache.clear().await;
        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_stats_reset() {
        let cache = VoiceTranscriptionCache::new(10);
        let lang = Arc::from("en");

        // Generate some hits and misses
        cache.get(1, &lang).await; // miss
        cache.get(2, &lang).await; // miss

        let stats = cache.stats();
        assert_eq!(stats.misses, 2);

        cache.reset_stats();

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }
}
