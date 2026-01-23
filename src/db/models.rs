use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Guild (server) configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Guild {
    pub id: i64,
    pub guild_id: String,
    pub name: String,
    pub default_language: String,
    pub enabled_channels: String, // JSON array of channel IDs
    pub target_languages: String, // JSON array of language codes
    pub subscription_tier: String,
    pub subscription_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User language preferences
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserPreference {
    pub id: i64,
    pub user_id: String,
    pub guild_id: String,
    pub preferred_language: String,
    pub auto_translate: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Channel configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub channel_id: String,
    pub guild_id: String,
    pub enabled: bool,
    pub target_languages: String, // JSON array, overrides guild default
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Web view session
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WebSession {
    pub id: i64,
    pub session_id: String,
    pub user_id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Subscription tier enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionTier {
    Free,
    Basic,
    Pro,
    Enterprise,
}

impl SubscriptionTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Basic => "basic",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "basic" => Self::Basic,
            "pro" => Self::Pro,
            "enterprise" => Self::Enterprise,
            _ => Self::Free,
        }
    }

    /// Maximum languages for this tier
    pub fn max_languages(&self) -> usize {
        match self {
            Self::Free => 2,
            Self::Basic => 5,
            Self::Pro | Self::Enterprise => 50,
        }
    }

    /// Maximum messages per day for this tier
    pub fn max_messages_per_day(&self) -> u32 {
        match self {
            Self::Free => 100,
            Self::Basic => u32::MAX,
            Self::Pro => u32::MAX,
            Self::Enterprise => u32::MAX,
        }
    }

    /// Whether web view is available
    pub fn has_web_view(&self) -> bool {
        matches!(self, Self::Basic | Self::Pro | Self::Enterprise)
    }
}

impl std::fmt::Display for SubscriptionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Guild settings for easy manipulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildSettings {
    pub guild_id: String,
    pub name: String,
    pub default_language: String,
    pub enabled_channels: Vec<String>,
    pub target_languages: Vec<String>,
    pub subscription_tier: SubscriptionTier,
    pub subscription_expires_at: Option<DateTime<Utc>>,
}

impl From<Guild> for GuildSettings {
    fn from(guild: Guild) -> Self {
        Self {
            guild_id: guild.guild_id,
            name: guild.name,
            default_language: guild.default_language,
            enabled_channels: serde_json::from_str(&guild.enabled_channels).unwrap_or_default(),
            target_languages: serde_json::from_str(&guild.target_languages).unwrap_or_default(),
            subscription_tier: SubscriptionTier::from_str(&guild.subscription_tier),
            subscription_expires_at: guild.subscription_expires_at,
        }
    }
}

/// New guild creation request
#[derive(Debug, Clone)]
pub struct NewGuild {
    pub guild_id: String,
    pub name: String,
}

/// New user preference
#[derive(Debug, Clone)]
pub struct NewUserPreference {
    pub user_id: String,
    pub guild_id: String,
    pub preferred_language: String,
}

/// New web session
#[derive(Debug, Clone)]
pub struct NewWebSession {
    pub user_id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
}

impl NewWebSession {
    pub fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }
}

/// Voice channel translation settings
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VoiceChannelSettings {
    pub id: i64,
    pub guild_id: String,
    pub voice_channel_id: String,
    pub enabled: bool,
    pub target_language: String,
    pub enable_tts: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New voice channel settings
#[derive(Debug, Clone)]
pub struct NewVoiceChannelSettings {
    pub guild_id: String,
    pub voice_channel_id: String,
    pub target_language: String,
    pub enable_tts: bool,
}

/// Voice transcript settings - for posting transcripts to Discord threads
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VoiceTranscriptSettings {
    pub id: i64,
    pub guild_id: String,
    pub voice_channel_id: String,
    pub text_channel_id: String,
    pub enabled: bool,
    /// JSON array of language codes, e.g., ["en", "es", "fr"]
    pub languages: String,
    /// JSON map of language code to thread ID, e.g., {"en": "123456", "es": "789012"}
    pub thread_ids: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// New voice transcript settings
#[derive(Debug, Clone)]
pub struct NewVoiceTranscriptSettings {
    pub guild_id: String,
    pub voice_channel_id: String,
    pub text_channel_id: String,
    pub languages: Vec<String>,
}

impl VoiceTranscriptSettings {
    /// Get languages as Vec
    pub fn get_languages(&self) -> Vec<String> {
        serde_json::from_str(&self.languages).unwrap_or_default()
    }

    /// Get thread IDs as HashMap
    pub fn get_thread_ids(&self) -> std::collections::HashMap<String, String> {
        serde_json::from_str(&self.thread_ids).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- SubscriptionTier tests ---

    #[test]
    fn test_subscription_tier_from_str() {
        assert_eq!(SubscriptionTier::from_str("free"), SubscriptionTier::Free);
        assert_eq!(SubscriptionTier::from_str("basic"), SubscriptionTier::Basic);
        assert_eq!(SubscriptionTier::from_str("pro"), SubscriptionTier::Pro);
        assert_eq!(SubscriptionTier::from_str("enterprise"), SubscriptionTier::Enterprise);
        assert_eq!(SubscriptionTier::from_str("unknown"), SubscriptionTier::Free);
        assert_eq!(SubscriptionTier::from_str(""), SubscriptionTier::Free);
    }

    #[test]
    fn test_subscription_tier_as_str() {
        assert_eq!(SubscriptionTier::Free.as_str(), "free");
        assert_eq!(SubscriptionTier::Basic.as_str(), "basic");
        assert_eq!(SubscriptionTier::Pro.as_str(), "pro");
        assert_eq!(SubscriptionTier::Enterprise.as_str(), "enterprise");
    }

    #[test]
    fn test_subscription_tier_case_insensitive() {
        assert_eq!(SubscriptionTier::from_str("BASIC"), SubscriptionTier::Basic);
        assert_eq!(SubscriptionTier::from_str("Pro"), SubscriptionTier::Pro);
        assert_eq!(SubscriptionTier::from_str("Enterprise"), SubscriptionTier::Enterprise);
    }

    #[test]
    fn test_subscription_tier_max_languages() {
        assert_eq!(SubscriptionTier::Free.max_languages(), 2);
        assert_eq!(SubscriptionTier::Basic.max_languages(), 5);
        assert_eq!(SubscriptionTier::Pro.max_languages(), 50);
        assert_eq!(SubscriptionTier::Enterprise.max_languages(), 50);
    }

    #[test]
    fn test_subscription_tier_max_messages() {
        assert_eq!(SubscriptionTier::Free.max_messages_per_day(), 100);
        assert_eq!(SubscriptionTier::Basic.max_messages_per_day(), u32::MAX);
        assert_eq!(SubscriptionTier::Pro.max_messages_per_day(), u32::MAX);
    }

    #[test]
    fn test_subscription_tier_web_view() {
        assert!(!SubscriptionTier::Free.has_web_view());
        assert!(SubscriptionTier::Basic.has_web_view());
        assert!(SubscriptionTier::Pro.has_web_view());
        assert!(SubscriptionTier::Enterprise.has_web_view());
    }

    #[test]
    fn test_subscription_tier_display() {
        assert_eq!(format!("{}", SubscriptionTier::Free), "free");
        assert_eq!(format!("{}", SubscriptionTier::Pro), "pro");
    }

    // --- GuildSettings from Guild conversion ---

    #[test]
    fn test_guild_to_guild_settings() {
        let guild = Guild {
            id: 1,
            guild_id: "g123".to_string(),
            name: "Test Guild".to_string(),
            default_language: "es".to_string(),
            enabled_channels: r#"["ch1","ch2"]"#.to_string(),
            target_languages: r#"["en","es","fr"]"#.to_string(),
            subscription_tier: "pro".to_string(),
            subscription_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let settings: GuildSettings = guild.into();
        assert_eq!(settings.guild_id, "g123");
        assert_eq!(settings.default_language, "es");
        assert_eq!(settings.enabled_channels, vec!["ch1", "ch2"]);
        assert_eq!(settings.target_languages, vec!["en", "es", "fr"]);
        assert_eq!(settings.subscription_tier, SubscriptionTier::Pro);
    }

    #[test]
    fn test_guild_settings_invalid_json_defaults() {
        let guild = Guild {
            id: 1,
            guild_id: "g1".to_string(),
            name: "Test".to_string(),
            default_language: "en".to_string(),
            enabled_channels: "invalid json".to_string(),
            target_languages: "also invalid".to_string(),
            subscription_tier: "free".to_string(),
            subscription_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let settings: GuildSettings = guild.into();
        assert!(settings.enabled_channels.is_empty());
        assert!(settings.target_languages.is_empty());
    }

    // --- NewWebSession tests ---

    #[test]
    fn test_generate_session_id_uniqueness() {
        let id1 = NewWebSession::generate_session_id();
        let id2 = NewWebSession::generate_session_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_session_id_format() {
        let id = NewWebSession::generate_session_id();
        // UUID v4 format: 8-4-4-4-12
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
    }

    // --- VoiceTranscriptSettings tests ---

    #[test]
    fn test_voice_transcript_get_languages() {
        let settings = VoiceTranscriptSettings {
            id: 1,
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            text_channel_id: "tc1".to_string(),
            enabled: true,
            languages: r#"["en","es","fr"]"#.to_string(),
            thread_ids: "{}".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let langs = settings.get_languages();
        assert_eq!(langs, vec!["en", "es", "fr"]);
    }

    #[test]
    fn test_voice_transcript_get_thread_ids() {
        let settings = VoiceTranscriptSettings {
            id: 1,
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            text_channel_id: "tc1".to_string(),
            enabled: true,
            languages: r#"["en"]"#.to_string(),
            thread_ids: r#"{"en":"123456","es":"789012"}"#.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let ids = settings.get_thread_ids();
        assert_eq!(ids.get("en"), Some(&"123456".to_string()));
        assert_eq!(ids.get("es"), Some(&"789012".to_string()));
    }

    #[test]
    fn test_voice_transcript_invalid_json_defaults() {
        let settings = VoiceTranscriptSettings {
            id: 1,
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            text_channel_id: "tc1".to_string(),
            enabled: true,
            languages: "invalid".to_string(),
            thread_ids: "invalid".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(settings.get_languages().is_empty());
        assert!(settings.get_thread_ids().is_empty());
    }
}
