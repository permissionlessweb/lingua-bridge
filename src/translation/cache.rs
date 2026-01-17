use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Cache key for translations
#[derive(Clone, Debug, Eq)]
pub struct CacheKey {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.source_lang == other.source_lang
            && self.target_lang == other.target_lang
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.source_lang.hash(state);
        self.target_lang.hash(state);
    }
}

/// Cached translation entry
#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub translated_text: String,
    pub created_at: Instant,
}

impl CacheEntry {
    pub fn new(translated_text: String) -> Self {
        Self {
            translated_text,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// LRU-style translation cache with TTL
pub struct TranslationCache {
    cache: DashMap<CacheKey, CacheEntry>,
    ttl: Duration,
    max_size: usize,
}

impl TranslationCache {
    pub fn new(ttl_secs: u64, max_size: usize) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
            max_size,
        }
    }

    /// Get a cached translation if it exists and is not expired
    pub fn get(&self, key: &CacheKey) -> Option<String> {
        let entry = self.cache.get(key)?;
        if entry.is_expired(self.ttl) {
            drop(entry);
            self.cache.remove(key);
            None
        } else {
            Some(entry.translated_text.clone())
        }
    }

    /// Insert a translation into the cache
    pub fn insert(&self, key: CacheKey, translated_text: String) {
        // Simple eviction: if we're at max size, remove expired entries
        if self.cache.len() >= self.max_size {
            self.evict_expired();
        }

        // If still at max size, remove oldest entries (simple approach)
        if self.cache.len() >= self.max_size {
            let keys_to_remove: Vec<_> = self.cache
                .iter()
                .take(self.max_size / 10) // Remove 10% of entries
                .map(|r| r.key().clone())
                .collect();
            for k in keys_to_remove {
                self.cache.remove(&k);
            }
        }

        self.cache.insert(key, CacheEntry::new(translated_text));
    }

    /// Remove expired entries from the cache
    pub fn evict_expired(&self) {
        let keys_to_remove: Vec<_> = self.cache
            .iter()
            .filter(|r| r.value().is_expired(self.ttl))
            .map(|r| r.key().clone())
            .collect();

        for key in keys_to_remove {
            self.cache.remove(&key);
        }
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total = self.cache.len();
        let expired = self.cache
            .iter()
            .filter(|r| r.value().is_expired(self.ttl))
            .count();
        CacheStats {
            total_entries: total,
            expired_entries: expired,
            max_size: self.max_size,
            ttl_secs: self.ttl.as_secs(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub max_size: usize,
    pub ttl_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = TranslationCache::new(3600, 1000);
        let key = CacheKey {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };

        cache.insert(key.clone(), "Hola".to_string());
        assert_eq!(cache.get(&key), Some("Hola".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = TranslationCache::new(3600, 1000);
        let key = CacheKey {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };

        assert_eq!(cache.get(&key), None);
    }

    #[test]
    fn test_cache_expiry() {
        let cache = TranslationCache::new(0, 1000); // 0 second TTL
        let key = CacheKey {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };

        cache.insert(key.clone(), "Hola".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert_eq!(cache.get(&key), None);
    }
}
