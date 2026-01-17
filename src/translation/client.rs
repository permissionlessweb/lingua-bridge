use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::translation::cache::{CacheKey, TranslationCache};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Request body for translation
#[derive(Debug, Serialize)]
pub struct TranslateRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
}

/// Response from translation service
#[derive(Debug, Deserialize)]
pub struct TranslateResponse {
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    #[serde(default)]
    pub confidence: Option<f32>,
}

/// Request for language detection
#[derive(Debug, Serialize)]
pub struct DetectRequest {
    pub text: String,
}

/// Response from language detection
#[derive(Debug, Deserialize)]
pub struct DetectResponse {
    pub language: String,
    pub confidence: f32,
}

/// Health check response
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub model: String,
    #[serde(default)]
    pub model_loaded: bool,
}

/// Translation result with metadata
#[derive(Debug, Clone, Serialize)]
pub struct TranslationResult {
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub cached: bool,
}

/// Client for communicating with the inference sidecar
pub struct TranslationClient {
    http: Client,
    base_url: String,
    cache: Arc<TranslationCache>,
    max_retries: u32,
}

impl std::fmt::Debug for TranslationClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslationClient")
            .field("base_url", &self.base_url)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

impl TranslationClient {
    /// Create a new translation client from config
    pub fn new(config: &AppConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.inference.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        let cache = Arc::new(TranslationCache::new(
            config.translation.cache_ttl_secs,
            config.translation.cache_max_size,
        ));

        Self {
            http,
            base_url: config.inference.url.trim_end_matches('/').to_string(),
            cache,
            max_retries: config.inference.max_retries,
        }
    }

    /// Check if the inference service is healthy
    pub async fn health_check(&self) -> AppResult<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        debug!("Checking inference service health at {}", url);

        let response = self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Health check failed: {}", e);
                AppError::InferenceUnavailable
            })?;

        if !response.status().is_success() {
            return Err(AppError::InferenceUnavailable);
        }

        response.json().await.map_err(|e| {
            error!("Failed to parse health response: {}", e);
            AppError::InferenceUnavailable
        })
    }

    /// Detect the language of a text
    pub async fn detect_language(&self, text: &str) -> AppResult<DetectResponse> {
        let url = format!("{}/detect", self.base_url);
        let request = DetectRequest {
            text: text.to_string(),
        };

        debug!("Detecting language for text: {}...", &text.chars().take(50).collect::<String>());

        let response = self.http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Language detection request failed: {}", e);
                AppError::InferenceUnavailable
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Language detection failed with status {}: {}", status, body);
            return Err(AppError::LanguageDetection(format!("Service returned {}", status)));
        }

        response.json().await.map_err(|e| {
            error!("Failed to parse detection response: {}", e);
            AppError::LanguageDetection(e.to_string())
        })
    }

    /// Translate text from source language to target language
    pub async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> AppResult<TranslationResult> {
        // Skip translation if source and target are the same
        if source_lang == target_lang {
            return Ok(TranslationResult {
                original_text: text.to_string(),
                translated_text: text.to_string(),
                source_lang: source_lang.to_string(),
                target_lang: target_lang.to_string(),
                cached: false,
            });
        }

        // Check cache first
        let cache_key = CacheKey {
            text: text.to_string(),
            source_lang: source_lang.to_string(),
            target_lang: target_lang.to_string(),
        };

        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for translation");
            return Ok(TranslationResult {
                original_text: text.to_string(),
                translated_text: cached,
                source_lang: source_lang.to_string(),
                target_lang: target_lang.to_string(),
                cached: true,
            });
        }

        // Make request with retries
        let result = self.translate_with_retry(text, source_lang, target_lang).await?;

        // Cache the result
        self.cache.insert(cache_key, result.translated_text.clone());

        Ok(TranslationResult {
            original_text: text.to_string(),
            translated_text: result.translated_text,
            source_lang: result.source_lang,
            target_lang: result.target_lang,
            cached: false,
        })
    }

    /// Translate with automatic language detection
    pub async fn translate_auto(
        &self,
        text: &str,
        target_lang: &str,
    ) -> AppResult<TranslationResult> {
        // Detect source language
        let detection = self.detect_language(text).await?;
        info!("Detected language: {} (confidence: {:.2})", detection.language, detection.confidence);

        // Translate
        self.translate(text, &detection.language, target_lang).await
    }

    /// Translate to multiple target languages
    pub async fn translate_to_multiple(
        &self,
        text: &str,
        source_lang: &str,
        target_langs: &[String],
    ) -> Vec<AppResult<TranslationResult>> {
        let futures: Vec<_> = target_langs
            .iter()
            .map(|target| self.translate(text, source_lang, target))
            .collect();

        futures::future::join_all(futures).await
    }

    /// Internal: translate with retry logic
    async fn translate_with_retry(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> AppResult<TranslateResponse> {
        let url = format!("{}/translate", self.base_url);
        let request = TranslateRequest {
            text: text.to_string(),
            source_lang: source_lang.to_string(),
            target_lang: target_lang.to_string(),
        };

        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                warn!("Retrying translation (attempt {}/{}), waiting {:?}",
                    attempt, self.max_retries, delay);
                tokio::time::sleep(delay).await;
            }

            match self.http.post(&url).json(&request).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<TranslateResponse>().await {
                            Ok(result) => return Ok(result),
                            Err(e) => {
                                error!("Failed to parse translation response: {}", e);
                                last_error = Some(AppError::Translation(e.to_string()));
                            }
                        }
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        error!("Translation failed with status {}: {}", status, body);
                        last_error = Some(AppError::Translation(format!(
                            "Service returned {}: {}",
                            status, body
                        )));
                    }
                }
                Err(e) => {
                    error!("Translation request failed: {}", e);
                    last_error = Some(AppError::Http(e));
                }
            }
        }

        Err(last_error.unwrap_or(AppError::InferenceUnavailable))
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::translation::cache::CacheStats {
        self.cache.stats()
    }

    /// Clear the translation cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_request_serialization() {
        let request = TranslateRequest {
            text: "Hello".to_string(),
            source_lang: "en".to_string(),
            target_lang: "es".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("en"));
        assert!(json.contains("es"));
    }
}
