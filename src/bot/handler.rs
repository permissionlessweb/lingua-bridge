use crate::db::{DbPool, GuildRepo, UserPreferenceRepo, NewGuild};
use crate::translation::{TranslationClient, TranslationResult};
use crate::web::broadcast::BroadcastManager;
use poise::serenity_prelude::{self as serenity, Context, Message};
use std::sync::Arc;
use tracing::{error, info};

/// Handle incoming messages for auto-translation
pub async fn handle_message(
    ctx: &Context,
    msg: &Message,
    pool: &DbPool,
    translator: &TranslationClient,
    broadcast: &Arc<BroadcastManager>,
) {
    // Ignore bot messages
    if msg.author.bot {
        return;
    }

    // Ignore empty messages
    if msg.content.trim().is_empty() {
        return;
    }

    // Get guild ID
    let guild_id = match msg.guild_id {
        Some(id) => id.to_string(),
        None => return, // DMs not supported
    };

    let channel_id = msg.channel_id.to_string();
    let user_id = msg.author.id.to_string();

    // Check if channel is enabled for translation
    let is_enabled = match GuildRepo::is_channel_enabled(pool, &guild_id, &channel_id).await {
        Ok(enabled) => enabled,
        Err(e) => {
            error!("Failed to check channel status: {}", e);
            return;
        }
    };

    if !is_enabled {
        return;
    }

    // Get guild settings
    let settings = match GuildRepo::get_settings(pool, &guild_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return,
        Err(e) => {
            error!("Failed to get guild settings: {}", e);
            return;
        }
    };

    // Get user preference (optional)
    let user_pref = UserPreferenceRepo::get(pool, &user_id, &guild_id)
        .await
        .ok()
        .flatten();

    // Determine target languages
    let target_langs = if settings.target_languages.is_empty() {
        vec![settings.default_language.clone()]
    } else {
        settings.target_languages.clone()
    };

    // Translate message
    let results = translate_message(translator, &msg.content, &target_langs).await;

    // Process results
    for result in results {
        match result {
            Ok(translation) => {
                // Broadcast to web viewers
                broadcast.send_translation(
                    &channel_id,
                    &msg.author.name,
                    &msg.author.id.to_string(),
                    &translation,
                );

                // Send translation as Discord reply (optional, configurable)
                if should_send_discord_reply(&settings, &user_pref) {
                    send_translation_reply(ctx, msg, &translation).await;
                }
            }
            Err(e) => {
                error!("Translation failed: {}", e);
            }
        }
    }
}

/// Translate message to multiple languages
async fn translate_message(
    translator: &TranslationClient,
    text: &str,
    target_langs: &[String],
) -> Vec<Result<TranslationResult, crate::error::AppError>> {
    // First detect the source language
    let source_lang = match translator.detect_language(text).await {
        Ok(detection) => detection.language,
        Err(e) => {
            error!("Language detection failed: {}", e);
            return vec![Err(e)];
        }
    };

    // Translate to each target language (excluding source)
    let mut results = Vec::new();
    for target in target_langs {
        if target == &source_lang {
            continue;
        }
        let result = translator.translate(text, &source_lang, target).await;
        results.push(result);
    }

    results
}

/// Check if we should send a reply in Discord
fn should_send_discord_reply(
    _settings: &crate::db::GuildSettings,
    _user_pref: &Option<crate::db::UserPreference>,
) -> bool {
    // For MVP, always send replies
    // Later: make this configurable per guild/user
    true
}

/// Send translation as a Discord reply
async fn send_translation_reply(
    ctx: &Context,
    original_msg: &Message,
    translation: &TranslationResult,
) {
    // Skip if source and target are the same
    if translation.source_lang == translation.target_lang {
        return;
    }

    // Create embed for translation
    let embed = serenity::CreateEmbed::default()
        .description(&translation.translated_text)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "{} → {}",
            translation.source_lang.to_uppercase(),
            translation.target_lang.to_uppercase()
        )))
        .color(0x5865F2); // Discord blurple

    let builder = serenity::CreateMessage::default()
        .embed(embed)
        .reference_message(original_msg);

    if let Err(e) = original_msg.channel_id.send_message(&ctx.http, builder).await {
        error!("Failed to send translation reply: {}", e);
    }
}

/// Handle guild join event
pub async fn handle_guild_create(
    guild: &serenity::Guild,
    pool: &DbPool,
) {
    info!("Joined guild: {} ({})", guild.name, guild.id);

    let new_guild = NewGuild {
        guild_id: guild.id.to_string(),
        name: guild.name.clone(),
    };

    if let Err(e) = GuildRepo::upsert(pool, new_guild).await {
        error!("Failed to register guild: {}", e);
    }
}

/// Handle guild leave event
pub async fn handle_guild_delete(
    guild_id: serenity::GuildId,
) {
    info!("Left guild: {}", guild_id);
    // Optionally: clean up guild data
}
