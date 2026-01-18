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
