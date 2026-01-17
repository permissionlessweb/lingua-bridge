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
