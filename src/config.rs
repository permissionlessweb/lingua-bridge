use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::sync::OnceLock;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Admin transport configuration (for secure provisioning)
#[derive(Debug, Deserialize, Clone)]
pub struct AdminConfig {
    /// Admin's Ed25519 public key (base64 encoded)
    /// This is the ONLY trust anchor - only the holder of the corresponding
    /// private key can provision secrets to this bot.
    pub public_key: String,
    /// Port for admin provisioning endpoint
    #[serde(default = "default_admin_port")]
    pub port: u16,
    /// Host for admin provisioning endpoint
    #[serde(default = "default_admin_host")]
    pub host: String,
}

fn default_admin_port() -> u16 {
    9999
}

fn default_admin_host() -> String {
    "0.0.0.0".to_string()
}

/// Discord bot configuration (non-sensitive parts only)
/// The token is now provided via secure admin provisioning.
#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    /// Application ID (optional, for OAuth flows)
    #[serde(default)]
    pub application_id: Option<String>,
}

/// Inference service configuration
#[derive(Debug, Deserialize, Clone)]
pub struct InferenceConfig {
    pub url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

/// Web server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
    pub session_expiry_hours: u64,
    pub public_url: String,
}

/// Database configuration
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

/// Translation settings
#[derive(Debug, Deserialize, Clone)]
pub struct TranslationConfig {
    pub default_languages: Vec<String>,
    pub max_message_length: usize,
    pub cache_ttl_secs: u64,
    pub cache_max_size: usize,
}

/// Rate limiting settings
#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitsConfig {
    pub free_messages_per_minute: u32,
    pub paid_messages_per_minute: u32,
}

/// Voice translation settings
#[derive(Debug, Deserialize, Clone)]
pub struct VoiceConfig {
    /// WebSocket URL for voice inference service
    #[serde(default = "default_voice_url")]
    pub url: String,
    /// Enable TTS playback in Discord
    #[serde(default)]
    pub enable_tts_playback: bool,
    /// Audio buffer size in milliseconds
    #[serde(default = "default_buffer_ms")]
    pub buffer_ms: u32,
    /// VAD sensitivity threshold (0.0-1.0)
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,
    /// Default target language for voice translations
    #[serde(default = "default_voice_target_lang")]
    pub default_target_language: String,
}

fn default_voice_url() -> String {
    "ws://voice-inference:8001/voice".to_string()
}

fn default_buffer_ms() -> u32 {
    500
}

fn default_vad_threshold() -> f32 {
    0.5
}

fn default_voice_target_lang() -> String {
    "en".to_string()
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            url: default_voice_url(),
            enable_tts_playback: false,
            buffer_ms: default_buffer_ms(),
            vad_threshold: default_vad_threshold(),
            default_target_language: default_voice_target_lang(),
        }
    }
}

/// Root application configuration
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    /// Admin transport configuration (required)
    pub admin: AdminConfig,
    /// Discord configuration (non-sensitive)
    #[serde(default)]
    pub discord: DiscordConfig,
    pub inference: InferenceConfig,
    pub web: WebConfig,
    pub database: DatabaseConfig,
    pub translation: TranslationConfig,
    pub rate_limits: RateLimitsConfig,
    /// Voice translation configuration
    #[serde(default)]
    pub voice: VoiceConfig,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            application_id: None,
        }
    }
}

impl AppConfig {
    /// Load configuration from files and environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default config file
            .add_source(File::with_name("config/default").required(false))
            // Override with local config if present
            .add_source(File::with_name("config/local").required(false))
            // Override with environment variables (prefix: LINGUABRIDGE_)
            // e.g., LINGUABRIDGE_ADMIN__PUBLIC_KEY, LINGUABRIDGE_WEB__PORT
            .add_source(
                Environment::with_prefix("LINGUABRIDGE")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Initialize the global config singleton
    pub fn init() -> Result<&'static Self, ConfigError> {
        let config = Self::load()?;
        Ok(CONFIG.get_or_init(|| config))
    }

    /// Get reference to the global config
    pub fn get() -> &'static Self {
        CONFIG.get().expect("Config not initialized. Call AppConfig::init() first.")
    }
}

/// Helper to get inference URL with proper trailing slash handling
impl InferenceConfig {
    pub fn endpoint(&self, path: &str) -> String {
        let base = self.url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_endpoint() {
        let config = InferenceConfig {
            url: "http://localhost:8000/".to_string(),
            model: "test".to_string(),
            timeout_secs: 30,
            max_retries: 3,
        };
        assert_eq!(config.endpoint("/translate"), "http://localhost:8000/translate");
        assert_eq!(config.endpoint("translate"), "http://localhost:8000/translate");
    }
}
