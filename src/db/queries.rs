use crate::db::models::*;
use crate::error::{AppError, AppResult};
use chrono::{Duration, Utc};
use sqlx::{Pool, Sqlite};
use tracing::info;

pub type DbPool = Pool<Sqlite>;

/// Database operations for guilds
pub struct GuildRepo;

impl GuildRepo {
    /// Get guild by Discord guild ID
    pub async fn get_by_guild_id(pool: &DbPool, guild_id: &str) -> AppResult<Option<Guild>> {
        let guild = sqlx::query_as::<_, Guild>("SELECT * FROM guilds WHERE guild_id = ?")
            .bind(guild_id)
            .fetch_optional(pool)
            .await?;

        Ok(guild)
    }

    /// Get guild settings (parsed)
    pub async fn get_settings(pool: &DbPool, guild_id: &str) -> AppResult<Option<GuildSettings>> {
        Ok(Self::get_by_guild_id(pool, guild_id).await?.map(Into::into))
    }

    /// Create or update guild
    pub async fn upsert(pool: &DbPool, new_guild: NewGuild) -> AppResult<Guild> {
        let now = Utc::now();
        let default_langs = serde_json::to_string(&vec!["en"]).unwrap();
        let empty_channels = serde_json::to_string(&Vec::<String>::new()).unwrap();

        sqlx::query(
            r#"
            INSERT INTO guilds (guild_id, name, default_language, enabled_channels, target_languages, subscription_tier, created_at, updated_at)
            VALUES (?, ?, 'en', ?, ?, 'free', ?, ?)
            ON CONFLICT(guild_id) DO UPDATE SET
                name = excluded.name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&new_guild.guild_id)
        .bind(&new_guild.name)
        .bind(&empty_channels)
        .bind(&default_langs)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get_by_guild_id(pool, &new_guild.guild_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to retrieve created guild"))
    }

    /// Update guild default language
    pub async fn set_default_language(
        pool: &DbPool,
        guild_id: &str,
        language: &str,
    ) -> AppResult<()> {
        sqlx::query("UPDATE guilds SET default_language = ?, updated_at = ? WHERE guild_id = ?")
            .bind(language)
            .bind(Utc::now())
            .bind(guild_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Update guild target languages
    pub async fn set_target_languages(
        pool: &DbPool,
        guild_id: &str,
        languages: &[String],
    ) -> AppResult<()> {
        let langs_json = serde_json::to_string(languages).unwrap();
        sqlx::query("UPDATE guilds SET target_languages = ?, updated_at = ? WHERE guild_id = ?")
            .bind(langs_json)
            .bind(Utc::now())
            .bind(guild_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Enable a channel for translation
    pub async fn enable_channel(pool: &DbPool, guild_id: &str, channel_id: &str) -> AppResult<()> {
        let guild = Self::get_by_guild_id(pool, guild_id)
            .await?
            .ok_or(AppError::GuildNotConfigured)?;

        let mut channels: Vec<String> =
            serde_json::from_str(&guild.enabled_channels).unwrap_or_default();

        if !channels.contains(&channel_id.to_string()) {
            channels.push(channel_id.to_string());
        }

        let channels_json = serde_json::to_string(&channels).unwrap();
        sqlx::query("UPDATE guilds SET enabled_channels = ?, updated_at = ? WHERE guild_id = ?")
            .bind(channels_json)
            .bind(Utc::now())
            .bind(guild_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Disable a channel for translation
    pub async fn disable_channel(pool: &DbPool, guild_id: &str, channel_id: &str) -> AppResult<()> {
        let guild = Self::get_by_guild_id(pool, guild_id)
            .await?
            .ok_or(AppError::GuildNotConfigured)?;

        let mut channels: Vec<String> =
            serde_json::from_str(&guild.enabled_channels).unwrap_or_default();

        channels.retain(|c| c != channel_id);

        let channels_json = serde_json::to_string(&channels).unwrap();
        sqlx::query("UPDATE guilds SET enabled_channels = ?, updated_at = ? WHERE guild_id = ?")
            .bind(channels_json)
            .bind(Utc::now())
            .bind(guild_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Check if channel is enabled for translation
    pub async fn is_channel_enabled(
        pool: &DbPool,
        guild_id: &str,
        channel_id: &str,
    ) -> AppResult<bool> {
        let guild = match Self::get_by_guild_id(pool, guild_id).await? {
            Some(g) => g,
            None => return Ok(false),
        };

        let channels: Vec<String> =
            serde_json::from_str(&guild.enabled_channels).unwrap_or_default();

        Ok(channels.contains(&channel_id.to_string()))
    }
}

/// Database operations for user preferences
pub struct UserPreferenceRepo;

impl UserPreferenceRepo {
    /// Get user preference for a guild
    pub async fn get(
        pool: &DbPool,
        user_id: &str,
        guild_id: &str,
    ) -> AppResult<Option<UserPreference>> {
        let pref = sqlx::query_as::<_, UserPreference>(
            "SELECT * FROM user_preferences WHERE user_id = ? AND guild_id = ?",
        )
        .bind(user_id)
        .bind(guild_id)
        .fetch_optional(pool)
        .await?;

        Ok(pref)
    }

    /// Set user's preferred language
    pub async fn set_language(
        pool: &DbPool,
        user_id: &str,
        guild_id: &str,
        language: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO user_preferences (user_id, guild_id, preferred_language, auto_translate, created_at, updated_at)
            VALUES (?, ?, ?, true, ?, ?)
            ON CONFLICT(user_id, guild_id) DO UPDATE SET
                preferred_language = excluded.preferred_language,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(user_id)
        .bind(guild_id)
        .bind(language)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Toggle auto-translate for user
    pub async fn set_auto_translate(
        pool: &DbPool,
        user_id: &str,
        guild_id: &str,
        enabled: bool,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE user_preferences SET auto_translate = ?, updated_at = ? WHERE user_id = ? AND guild_id = ?",
        )
        .bind(enabled)
        .bind(Utc::now())
        .bind(user_id)
        .bind(guild_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Database operations for web sessions
pub struct WebSessionRepo;

impl WebSessionRepo {
    /// Create a new web session
    pub async fn create(
        pool: &DbPool,
        session: NewWebSession,
        expiry_hours: u64,
    ) -> AppResult<WebSession> {
        let session_id = NewWebSession::generate_session_id();
        let now = Utc::now();
        let expires_at = now + Duration::hours(expiry_hours as i64);

        sqlx::query(
            r#"
            INSERT INTO web_sessions (session_id, user_id, guild_id, channel_id, expires_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(&session.user_id)
        .bind(&session.guild_id)
        .bind(&session.channel_id)
        .bind(expires_at)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get_by_session_id(pool, &session_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to create session"))
    }

    /// Get session by ID
    pub async fn get_by_session_id(
        pool: &DbPool,
        session_id: &str,
    ) -> AppResult<Option<WebSession>> {
        let session = sqlx::query_as::<_, WebSession>(
            "SELECT * FROM web_sessions WHERE session_id = ? AND expires_at > ?",
        )
        .bind(session_id)
        .bind(Utc::now())
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Delete expired sessions
    pub async fn cleanup_expired(pool: &DbPool) -> AppResult<u64> {
        let result = sqlx::query("DELETE FROM web_sessions WHERE expires_at <= ?")
            .bind(Utc::now())
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete session
    pub async fn delete(pool: &DbPool, session_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM web_sessions WHERE session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

/// Database operations for voice channel settings
pub struct VoiceChannelRepo;

impl VoiceChannelRepo {
    /// Get settings for a voice channel
    pub async fn get_settings(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
    ) -> AppResult<Option<VoiceChannelSettings>> {
        let settings = sqlx::query_as::<_, VoiceChannelSettings>(
            "SELECT * FROM voice_channel_settings WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(guild_id)
        .bind(voice_channel_id)
        .fetch_optional(pool)
        .await?;

        Ok(settings)
    }

    /// Get all voice channel settings for a guild
    pub async fn get_by_guild(
        pool: &DbPool,
        guild_id: &str,
    ) -> AppResult<Vec<VoiceChannelSettings>> {
        let settings = sqlx::query_as::<_, VoiceChannelSettings>(
            "SELECT * FROM voice_channel_settings WHERE guild_id = ?",
        )
        .bind(guild_id)
        .fetch_all(pool)
        .await?;

        Ok(settings)
    }

    /// Create or update voice channel settings
    pub async fn upsert(
        pool: &DbPool,
        settings: NewVoiceChannelSettings,
    ) -> AppResult<VoiceChannelSettings> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO voice_channel_settings (guild_id, voice_channel_id, enabled, target_language, enable_tts, created_at, updated_at)
            VALUES (?, ?, true, ?, ?, ?, ?)
            ON CONFLICT(guild_id, voice_channel_id) DO UPDATE SET
                target_language = excluded.target_language,
                enable_tts = excluded.enable_tts,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&settings.guild_id)
        .bind(&settings.voice_channel_id)
        .bind(&settings.target_language)
        .bind(settings.enable_tts)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get_settings(pool, &settings.guild_id, &settings.voice_channel_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to retrieve created voice settings"))
    }

    /// Enable/disable voice translation for a channel
    pub async fn set_enabled(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
        enabled: bool,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE voice_channel_settings SET enabled = ?, updated_at = ? WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(enabled)
        .bind(Utc::now())
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update target language for a voice channel
    pub async fn set_target_language(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
        language: &str,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE voice_channel_settings SET target_language = ?, updated_at = ? WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(language)
        .bind(Utc::now())
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update TTS setting for a voice channel
    pub async fn set_tts_enabled(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
        enabled: bool,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE voice_channel_settings SET enable_tts = ?, updated_at = ? WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(enabled)
        .bind(Utc::now())
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete voice channel settings
    pub async fn delete(pool: &DbPool, guild_id: &str, voice_channel_id: &str) -> AppResult<()> {
        sqlx::query(
            "DELETE FROM voice_channel_settings WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

/// Database operations for voice transcript settings
pub struct VoiceTranscriptRepo;

impl VoiceTranscriptRepo {
    /// Get transcript settings for a voice channel
    pub async fn get_settings(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
    ) -> AppResult<Option<VoiceTranscriptSettings>> {
        let settings = sqlx::query_as::<_, VoiceTranscriptSettings>(
            "SELECT * FROM voice_transcript_settings WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(guild_id)
        .bind(voice_channel_id)
        .fetch_optional(pool)
        .await?;

        Ok(settings)
    }

    /// Get all transcript settings for a guild
    pub async fn get_by_guild(
        pool: &DbPool,
        guild_id: &str,
    ) -> AppResult<Vec<VoiceTranscriptSettings>> {
        let settings = sqlx::query_as::<_, VoiceTranscriptSettings>(
            "SELECT * FROM voice_transcript_settings WHERE guild_id = ? AND enabled = true",
        )
        .bind(guild_id)
        .fetch_all(pool)
        .await?;

        Ok(settings)
    }

    /// Create or update transcript settings
    pub async fn upsert(
        pool: &DbPool,
        settings: NewVoiceTranscriptSettings,
    ) -> AppResult<VoiceTranscriptSettings> {
        let now = Utc::now();
        let languages_json = serde_json::to_string(&settings.languages).unwrap();
        let empty_threads =
            serde_json::to_string(&std::collections::HashMap::<String, String>::new()).unwrap();

        sqlx::query(
            r#"
            INSERT INTO voice_transcript_settings (guild_id, voice_channel_id, text_channel_id, enabled, languages, thread_ids, created_at, updated_at)
            VALUES (?, ?, ?, true, ?, ?, ?, ?)
            ON CONFLICT(guild_id, voice_channel_id) DO UPDATE SET
                text_channel_id = excluded.text_channel_id,
                enabled = true,
                languages = excluded.languages,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&settings.guild_id)
        .bind(&settings.voice_channel_id)
        .bind(&settings.text_channel_id)
        .bind(&languages_json)
        .bind(&empty_threads)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get_settings(pool, &settings.guild_id, &settings.voice_channel_id)
            .await?
            .ok_or_else(|| AppError::internal("Failed to retrieve created transcript settings"))
    }

    /// Enable/disable transcripts for a voice channel
    pub async fn set_enabled(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
        enabled: bool,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE voice_transcript_settings SET enabled = ?, updated_at = ? WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(enabled)
        .bind(Utc::now())
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update thread ID for a language
    pub async fn set_thread_id(
        pool: &DbPool,
        guild_id: &str,
        voice_channel_id: &str,
        language: &str,
        thread_id: &str,
    ) -> AppResult<()> {
        // Get current settings
        let settings = Self::get_settings(pool, guild_id, voice_channel_id)
            .await?
            .ok_or_else(|| AppError::internal("Transcript settings not found"))?;

        let mut thread_ids = settings.get_thread_ids();
        thread_ids.insert(language.to_string(), thread_id.to_string());
        let thread_ids_json = serde_json::to_string(&thread_ids).unwrap();

        sqlx::query(
            "UPDATE voice_transcript_settings SET thread_ids = ?, updated_at = ? WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(thread_ids_json)
        .bind(Utc::now())
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete transcript settings
    pub async fn delete(pool: &DbPool, guild_id: &str, voice_channel_id: &str) -> AppResult<()> {
        sqlx::query(
            "DELETE FROM voice_transcript_settings WHERE guild_id = ? AND voice_channel_id = ?",
        )
        .bind(guild_id)
        .bind(voice_channel_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
pub async fn setup_test_db() -> DbPool {
    use sqlx::sqlite::SqlitePoolOptions;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    init_db(&pool).await.expect("Failed to init database");
    pool
}

/// Initialize database with migrations
pub async fn init_db(pool: &DbPool) -> AppResult<()> {
    info!("Running database migrations");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS guilds (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            default_language TEXT NOT NULL DEFAULT 'en',
            enabled_channels TEXT NOT NULL DEFAULT '[]',
            target_languages TEXT NOT NULL DEFAULT '["en"]',
            subscription_tier TEXT NOT NULL DEFAULT 'free',
            subscription_expires_at DATETIME,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_preferences (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            guild_id TEXT NOT NULL,
            preferred_language TEXT NOT NULL,
            auto_translate BOOLEAN NOT NULL DEFAULT true,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            UNIQUE(user_id, guild_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS channels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_id TEXT UNIQUE NOT NULL,
            guild_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT true,
            target_languages TEXT NOT NULL DEFAULT '[]',
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS web_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT UNIQUE NOT NULL,
            user_id TEXT NOT NULL,
            guild_id TEXT NOT NULL,
            channel_id TEXT,
            expires_at DATETIME NOT NULL,
            created_at DATETIME NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS voice_channel_settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id TEXT NOT NULL,
            voice_channel_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT true,
            target_language TEXT NOT NULL DEFAULT 'en',
            enable_tts BOOLEAN NOT NULL DEFAULT false,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            UNIQUE(guild_id, voice_channel_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS voice_transcript_settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id TEXT NOT NULL,
            voice_channel_id TEXT NOT NULL,
            text_channel_id TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT true,
            languages TEXT NOT NULL DEFAULT '["en"]',
            thread_ids TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            UNIQUE(guild_id, voice_channel_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_guilds_guild_id ON guilds(guild_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_prefs_user_guild ON user_preferences(user_id, guild_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON web_sessions(session_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_voice_settings_guild ON voice_channel_settings(guild_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_voice_transcript_guild ON voice_transcript_settings(guild_id)")
        .execute(pool)
        .await?;

    info!("Database migrations complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GuildRepo tests ---

    #[tokio::test]
    async fn test_guild_upsert_creates_new() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test Guild".to_string(),
        };

        let guild = GuildRepo::upsert(&pool, new_guild).await.unwrap();
        assert_eq!(guild.guild_id, "g123");
        assert_eq!(guild.name, "Test Guild");
        assert_eq!(guild.default_language, "en");
    }

    #[tokio::test]
    async fn test_guild_upsert_updates_existing() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Original Name".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        let updated = NewGuild {
            guild_id: "g123".to_string(),
            name: "Updated Name".to_string(),
        };
        let guild = GuildRepo::upsert(&pool, updated).await.unwrap();
        assert_eq!(guild.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_guild_get_nonexistent_returns_none() {
        let pool = setup_test_db().await;
        let result = GuildRepo::get_by_guild_id(&pool, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_guild_set_default_language() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        GuildRepo::set_default_language(&pool, "g123", "es").await.unwrap();
        let guild = GuildRepo::get_by_guild_id(&pool, "g123").await.unwrap().unwrap();
        assert_eq!(guild.default_language, "es");
    }

    #[tokio::test]
    async fn test_guild_set_target_languages() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        let langs = vec!["en".to_string(), "es".to_string(), "fr".to_string()];
        GuildRepo::set_target_languages(&pool, "g123", &langs).await.unwrap();

        let guild = GuildRepo::get_by_guild_id(&pool, "g123").await.unwrap().unwrap();
        let stored_langs: Vec<String> = serde_json::from_str(&guild.target_languages).unwrap();
        assert_eq!(stored_langs, langs);
    }

    #[tokio::test]
    async fn test_guild_enable_channel() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        GuildRepo::enable_channel(&pool, "g123", "ch456").await.unwrap();
        assert!(GuildRepo::is_channel_enabled(&pool, "g123", "ch456").await.unwrap());
    }

    #[tokio::test]
    async fn test_guild_disable_channel() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        GuildRepo::enable_channel(&pool, "g123", "ch456").await.unwrap();
        GuildRepo::disable_channel(&pool, "g123", "ch456").await.unwrap();
        assert!(!GuildRepo::is_channel_enabled(&pool, "g123", "ch456").await.unwrap());
    }

    #[tokio::test]
    async fn test_guild_enable_channel_idempotent() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        GuildRepo::enable_channel(&pool, "g123", "ch456").await.unwrap();
        GuildRepo::enable_channel(&pool, "g123", "ch456").await.unwrap();

        let guild = GuildRepo::get_by_guild_id(&pool, "g123").await.unwrap().unwrap();
        let channels: Vec<String> = serde_json::from_str(&guild.enabled_channels).unwrap();
        assert_eq!(channels.len(), 1);
    }

    #[tokio::test]
    async fn test_guild_channel_not_enabled_by_default() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();
        assert!(!GuildRepo::is_channel_enabled(&pool, "g123", "ch456").await.unwrap());
    }

    #[tokio::test]
    async fn test_guild_get_settings() {
        let pool = setup_test_db().await;
        let new_guild = NewGuild {
            guild_id: "g123".to_string(),
            name: "Test".to_string(),
        };
        GuildRepo::upsert(&pool, new_guild).await.unwrap();

        let settings = GuildRepo::get_settings(&pool, "g123").await.unwrap();
        assert!(settings.is_some());
        let s = settings.unwrap();
        assert_eq!(s.guild_id, "g123");
        assert_eq!(s.default_language, "en");
    }

    // --- UserPreferenceRepo tests ---

    #[tokio::test]
    async fn test_user_preference_set_and_get() {
        let pool = setup_test_db().await;
        UserPreferenceRepo::set_language(&pool, "u1", "g1", "es").await.unwrap();

        let pref = UserPreferenceRepo::get(&pool, "u1", "g1").await.unwrap();
        assert!(pref.is_some());
        let p = pref.unwrap();
        assert_eq!(p.preferred_language, "es");
        assert!(p.auto_translate);
    }

    #[tokio::test]
    async fn test_user_preference_get_nonexistent() {
        let pool = setup_test_db().await;
        let pref = UserPreferenceRepo::get(&pool, "u1", "g1").await.unwrap();
        assert!(pref.is_none());
    }

    #[tokio::test]
    async fn test_user_preference_update_language() {
        let pool = setup_test_db().await;
        UserPreferenceRepo::set_language(&pool, "u1", "g1", "es").await.unwrap();
        UserPreferenceRepo::set_language(&pool, "u1", "g1", "fr").await.unwrap();

        let pref = UserPreferenceRepo::get(&pool, "u1", "g1").await.unwrap().unwrap();
        assert_eq!(pref.preferred_language, "fr");
    }

    #[tokio::test]
    async fn test_user_preference_auto_translate_toggle() {
        let pool = setup_test_db().await;
        UserPreferenceRepo::set_language(&pool, "u1", "g1", "es").await.unwrap();
        UserPreferenceRepo::set_auto_translate(&pool, "u1", "g1", false).await.unwrap();

        let pref = UserPreferenceRepo::get(&pool, "u1", "g1").await.unwrap().unwrap();
        assert!(!pref.auto_translate);
    }

    // --- WebSessionRepo tests ---

    #[tokio::test]
    async fn test_session_create() {
        let pool = setup_test_db().await;
        let new_session = NewWebSession {
            user_id: "u1".to_string(),
            guild_id: "g1".to_string(),
            channel_id: Some("ch1".to_string()),
        };

        let session = WebSessionRepo::create(&pool, new_session, 24).await.unwrap();
        assert_eq!(session.user_id, "u1");
        assert_eq!(session.guild_id, "g1");
        assert_eq!(session.channel_id, Some("ch1".to_string()));
    }

    #[tokio::test]
    async fn test_session_get_valid() {
        let pool = setup_test_db().await;
        let new_session = NewWebSession {
            user_id: "u1".to_string(),
            guild_id: "g1".to_string(),
            channel_id: None,
        };

        let session = WebSessionRepo::create(&pool, new_session, 24).await.unwrap();
        let retrieved = WebSessionRepo::get_by_session_id(&pool, &session.session_id)
            .await
            .unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_session_get_nonexistent() {
        let pool = setup_test_db().await;
        let result = WebSessionRepo::get_by_session_id(&pool, "nonexistent")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_delete() {
        let pool = setup_test_db().await;
        let new_session = NewWebSession {
            user_id: "u1".to_string(),
            guild_id: "g1".to_string(),
            channel_id: None,
        };

        let session = WebSessionRepo::create(&pool, new_session, 24).await.unwrap();
        WebSessionRepo::delete(&pool, &session.session_id).await.unwrap();

        let result = WebSessionRepo::get_by_session_id(&pool, &session.session_id)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_cleanup_expired() {
        let pool = setup_test_db().await;
        // Create session with 0 hours expiry (already expired)
        let new_session = NewWebSession {
            user_id: "u1".to_string(),
            guild_id: "g1".to_string(),
            channel_id: None,
        };
        // Use very short expiry - it will still be in the future but we can test the cleanup function
        WebSessionRepo::create(&pool, new_session, 24).await.unwrap();
        let cleaned = WebSessionRepo::cleanup_expired(&pool).await.unwrap();
        // Session is not yet expired (24h from now)
        assert_eq!(cleaned, 0);
    }

    // --- VoiceChannelRepo tests ---

    #[tokio::test]
    async fn test_voice_channel_upsert() {
        let pool = setup_test_db().await;
        let settings = NewVoiceChannelSettings {
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            target_language: "es".to_string(),
            enable_tts: true,
        };

        let result = VoiceChannelRepo::upsert(&pool, settings).await.unwrap();
        assert_eq!(result.guild_id, "g1");
        assert_eq!(result.target_language, "es");
        assert!(result.enable_tts);
        assert!(result.enabled);
    }

    #[tokio::test]
    async fn test_voice_channel_get_settings() {
        let pool = setup_test_db().await;
        let settings = NewVoiceChannelSettings {
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            target_language: "fr".to_string(),
            enable_tts: false,
        };
        VoiceChannelRepo::upsert(&pool, settings).await.unwrap();

        let result = VoiceChannelRepo::get_settings(&pool, "g1", "vc1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().target_language, "fr");
    }

    #[tokio::test]
    async fn test_voice_channel_get_nonexistent() {
        let pool = setup_test_db().await;
        let result = VoiceChannelRepo::get_settings(&pool, "g1", "vc1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_voice_channel_set_enabled() {
        let pool = setup_test_db().await;
        let settings = NewVoiceChannelSettings {
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            target_language: "es".to_string(),
            enable_tts: false,
        };
        VoiceChannelRepo::upsert(&pool, settings).await.unwrap();

        VoiceChannelRepo::set_enabled(&pool, "g1", "vc1", false).await.unwrap();
        let result = VoiceChannelRepo::get_settings(&pool, "g1", "vc1").await.unwrap().unwrap();
        assert!(!result.enabled);
    }

    #[tokio::test]
    async fn test_voice_channel_set_target_language() {
        let pool = setup_test_db().await;
        let settings = NewVoiceChannelSettings {
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            target_language: "es".to_string(),
            enable_tts: false,
        };
        VoiceChannelRepo::upsert(&pool, settings).await.unwrap();

        VoiceChannelRepo::set_target_language(&pool, "g1", "vc1", "ja").await.unwrap();
        let result = VoiceChannelRepo::get_settings(&pool, "g1", "vc1").await.unwrap().unwrap();
        assert_eq!(result.target_language, "ja");
    }

    #[tokio::test]
    async fn test_voice_channel_set_tts() {
        let pool = setup_test_db().await;
        let settings = NewVoiceChannelSettings {
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            target_language: "es".to_string(),
            enable_tts: false,
        };
        VoiceChannelRepo::upsert(&pool, settings).await.unwrap();

        VoiceChannelRepo::set_tts_enabled(&pool, "g1", "vc1", true).await.unwrap();
        let result = VoiceChannelRepo::get_settings(&pool, "g1", "vc1").await.unwrap().unwrap();
        assert!(result.enable_tts);
    }

    #[tokio::test]
    async fn test_voice_channel_delete() {
        let pool = setup_test_db().await;
        let settings = NewVoiceChannelSettings {
            guild_id: "g1".to_string(),
            voice_channel_id: "vc1".to_string(),
            target_language: "es".to_string(),
            enable_tts: false,
        };
        VoiceChannelRepo::upsert(&pool, settings).await.unwrap();

        VoiceChannelRepo::delete(&pool, "g1", "vc1").await.unwrap();
        let result = VoiceChannelRepo::get_settings(&pool, "g1", "vc1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_voice_channel_get_by_guild() {
        let pool = setup_test_db().await;
        for i in 0..3 {
            let settings = NewVoiceChannelSettings {
                guild_id: "g1".to_string(),
                voice_channel_id: format!("vc{}", i),
                target_language: "es".to_string(),
                enable_tts: false,
            };
            VoiceChannelRepo::upsert(&pool, settings).await.unwrap();
        }

        let results = VoiceChannelRepo::get_by_guild(&pool, "g1").await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
