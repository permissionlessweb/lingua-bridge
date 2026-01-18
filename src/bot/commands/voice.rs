//! Voice channel translation commands.

use crate::bot::Data;
use crate::db::{NewVoiceTranscriptSettings, VoiceTranscriptRepo};
use crate::translation::Language;
use crate::voice::{VoiceClientConfig, VoiceManager};
use poise::serenity_prelude as serenity;
use std::sync::Arc;
use tracing::{error, info};

// Re-export for convenience

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Voice translation commands
#[poise::command(
    slash_command,
    guild_only,
    subcommands("join", "leave", "status", "url", "transcript"),
    subcommand_required
)]
pub async fn voice(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Join a voice channel and start translating
#[poise::command(slash_command, guild_only)]
pub async fn join(
    ctx: Context<'_>,
    #[description = "Voice channel to join (joins your current channel if not specified)"]
    channel: Option<serenity::GuildChannel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    // Determine which channel to join
    let channel_id = if let Some(ch) = channel {
        if ch.kind != serenity::ChannelType::Voice {
            return Err("Please specify a voice channel".into());
        }
        ch.id
    } else {
        // Try to get user's current voice channel
        let guild = ctx
            .guild()
            .ok_or("Could not get guild info")?
            .clone();

        let user_voice_state = guild.voice_states.get(&ctx.author().id);
        match user_voice_state.and_then(|vs| vs.channel_id) {
            Some(id) => id,
            None => {
                return Err("You're not in a voice channel. Either join one or specify a channel.".into());
            }
        }
    };

    ctx.defer().await?;

    // Get Songbird manager
    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice client not initialized")?;

    // Check if already in a channel
    if let Some(call) = manager.get(guild_id) {
        let current_channel = call.lock().await.current_channel();
        if current_channel.is_some() {
            let current_id = current_channel.unwrap();
            if current_id.0.get() == channel_id.get() {
                return Err("Already in this voice channel!".into());
            }
            // Leave current channel first
            manager.remove(guild_id).await?;
        }
    }

    // Join the channel
    let call = manager.join(guild_id, channel_id).await.map_err(|e| {
        error!(error = %e, "Failed to join voice channel");
        format!("Failed to join voice channel: {}", e)
    })?;

    // Set up voice receive handler
    let config = crate::config::AppConfig::get();
    let voice_config = VoiceClientConfig {
        url: config.voice.url.clone(),
        ..Default::default()
    };

    let voice_manager = Arc::new(VoiceManager::new(manager.clone(), voice_config));
    let handler = voice_manager.get_or_create_handler(guild_id.get(), channel_id.get());

    // Register event handler for receiving audio
    // We need to use Arc::unwrap_or_clone to get the handler since songbird expects ownership
    {
        let mut call_lock = call.lock().await;

        // Create separate handlers for each event type since songbird takes ownership
        let handler1 = (*handler).clone();
        let handler2 = (*handler).clone();
        let handler3 = (*handler).clone();

        // Enable receiving audio
        call_lock.add_global_event(
            songbird::CoreEvent::SpeakingStateUpdate.into(),
            handler1,
        );
        call_lock.add_global_event(
            songbird::CoreEvent::VoiceTick.into(),
            handler2,
        );
        call_lock.add_global_event(
            songbird::CoreEvent::ClientDisconnect.into(),
            handler3,
        );
    }

    info!(
        guild_id = guild_id.get(),
        channel_id = channel_id.get(),
        "Joined voice channel for translation"
    );

    let embed = serenity::CreateEmbed::default()
        .title("Voice Translation Active")
        .description(format!(
            "Joined <#{}>\n\nSpeak in the voice channel and I'll transcribe and translate your speech.\n\nTarget language: **{}**",
            channel_id,
            config.voice.default_target_language.to_uppercase()
        ))
        .field(
            "TTS Playback",
            if config.voice.enable_tts_playback {
                "Enabled"
            } else {
                "Disabled"
            },
            true,
        )
        .footer(serenity::CreateEmbedFooter::new(
            "Use /voice leave to stop",
        ))
        .color(0x57F287);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Leave the current voice channel
#[poise::command(slash_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice client not initialized")?;

    if manager.get(guild_id).is_none() {
        return Err("Not in a voice channel".into());
    }

    manager.remove(guild_id).await?;

    info!(guild_id = guild_id.get(), "Left voice channel");

    let embed = serenity::CreateEmbed::default()
        .title("Voice Translation Stopped")
        .description("Left the voice channel. Use `/voice join` to start again.")
        .color(0xED4245);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Check voice translation status
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice client not initialized")?;

    let embed = if let Some(call) = manager.get(guild_id) {
        let call = call.lock().await;
        let channel = call.current_channel();

        if let Some(channel_id) = channel {
            let config = crate::config::AppConfig::get();

            serenity::CreateEmbed::default()
                .title("Voice Translation Status")
                .description(format!("Currently in <#{}>", channel_id.0.get()))
                .field(
                    "Target Language",
                    config.voice.default_target_language.to_uppercase(),
                    true,
                )
                .field(
                    "TTS Playback",
                    if config.voice.enable_tts_playback {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                    true,
                )
                .field(
                    "Inference Service",
                    &config.voice.url,
                    false,
                )
                .color(0x57F287)
        } else {
            serenity::CreateEmbed::default()
                .title("Voice Translation Status")
                .description("Not currently in a voice channel")
                .color(0xFEE75C)
        }
    } else {
        serenity::CreateEmbed::default()
            .title("Voice Translation Status")
            .description("Not currently in a voice channel")
            .footer(serenity::CreateEmbedFooter::new(
                "Use /voice join to start translating",
            ))
            .color(0xFEE75C)
    };

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Configure voice translation settings for this server
#[poise::command(slash_command, guild_only)]
pub async fn voiceconfig(
    ctx: Context<'_>,
    #[description = "Target language for translations (e.g., 'en', 'es', 'ja')"]
    target_language: Option<String>,
    #[description = "Enable TTS playback of translations"] enable_tts: Option<bool>,
) -> Result<(), Error> {
    let _guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    let mut updates = Vec::new();

    if let Some(lang) = &target_language {
        if Language::from_code(lang).is_none() {
            return Err(format!(
                "Unknown language: {}. Use ISO 639-1 codes like 'en', 'es', 'fr'.",
                lang
            )
            .into());
        }
        updates.push(format!("Target language: **{}**", lang.to_uppercase()));
    }

    if let Some(tts) = enable_tts {
        updates.push(format!(
            "TTS playback: **{}**",
            if tts { "Enabled" } else { "Disabled" }
        ));
    }

    if updates.is_empty() {
        let config = crate::config::AppConfig::get();
        let embed = serenity::CreateEmbed::default()
            .title("Voice Configuration")
            .description("Current settings for this server:")
            .field(
                "Target Language",
                config.voice.default_target_language.to_uppercase(),
                true,
            )
            .field(
                "TTS Playback",
                if config.voice.enable_tts_playback {
                    "Enabled"
                } else {
                    "Disabled"
                },
                true,
            )
            .footer(serenity::CreateEmbedFooter::new(
                "Use /voiceconfig with options to change settings",
            ))
            .color(0x5865F2);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        // TODO: Save to database per-guild
        let embed = serenity::CreateEmbed::default()
            .title("Voice Configuration Updated")
            .description(updates.join("\n"))
            .footer(serenity::CreateEmbedFooter::new(
                "Settings apply to new voice sessions",
            ))
            .color(0x57F287);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}

/// Get the public web URL for a voice channel
#[poise::command(slash_command, guild_only)]
pub async fn url(
    ctx: Context<'_>,
    #[description = "Voice channel to get URL for (uses your current channel if not specified)"]
    channel: Option<serenity::GuildChannel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    // Determine which channel to get URL for
    let channel_id = if let Some(ch) = channel {
        if ch.kind != serenity::ChannelType::Voice {
            return Err("Please specify a voice channel".into());
        }
        ch.id
    } else {
        // Try to get user's current voice channel
        let guild = ctx
            .guild()
            .ok_or("Could not get guild info")?
            .clone();

        let user_voice_state = guild.voice_states.get(&ctx.author().id);
        match user_voice_state.and_then(|vs| vs.channel_id) {
            Some(id) => id,
            None => {
                return Err("You're not in a voice channel. Either join one or specify a channel.".into());
            }
        }
    };

    let config = crate::config::AppConfig::get();
    let public_url = format!(
        "{}/voice/{}/{}",
        config.web.public_url,
        guild_id.get(),
        channel_id.get()
    );

    let embed = serenity::CreateEmbed::default()
        .title("Voice Translation Web View")
        .description(format!(
            "Share this link to view real-time voice translations:\n\n**{}**",
            public_url
        ))
        .field(
            "Channel",
            format!("<#{}>", channel_id),
            true,
        )
        .field(
            "Features",
            "• Live transcription\n• Translation display\n• TTS audio playback\n• Relative timestamps",
            false,
        )
        .footer(serenity::CreateEmbedFooter::new(
            "Anyone with this link can view translations",
        ))
        .color(0x5865F2);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Enable or disable transcript posting to Discord threads
#[poise::command(slash_command, guild_only)]
pub async fn transcript(
    ctx: Context<'_>,
    #[description = "Enable transcript posting"] enable: bool,
    #[description = "Text channel to post transcripts in"] text_channel: Option<serenity::GuildChannel>,
    #[description = "Comma-separated list of target languages (e.g., 'en,es,fr')"] languages: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    // Determine which voice channel this applies to
    let guild = ctx
        .guild()
        .ok_or("Could not get guild info")?
        .clone();

    let voice_channel_id = {
        let user_voice_state = guild.voice_states.get(&ctx.author().id);
        match user_voice_state.and_then(|vs| vs.channel_id) {
            Some(id) => id,
            None => {
                return Err("You must be in a voice channel to configure transcripts.".into());
            }
        }
    };

    let pool = &ctx.data().pool;

    if !enable {
        // Disable transcripts in database
        VoiceTranscriptRepo::set_enabled(
            pool,
            &guild_id.to_string(),
            &voice_channel_id.to_string(),
            false,
        )
        .await
        .ok(); // Ignore error if settings don't exist

        let embed = serenity::CreateEmbed::default()
            .title("Voice Transcripts Disabled")
            .description(format!(
                "Transcripts for <#{}> will no longer be posted.",
                voice_channel_id
            ))
            .color(0xED4245);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    // Parse languages
    let lang_list = languages
        .as_deref()
        .unwrap_or("en")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect::<Vec<_>>();

    // Validate languages
    for lang in &lang_list {
        if crate::translation::Language::from_code(lang).is_none() {
            return Err(format!(
                "Unknown language: {}. Use ISO 639-1 codes like 'en', 'es', 'fr'.",
                lang
            )
            .into());
        }
    }

    // Determine text channel for transcripts
    let transcript_channel = if let Some(ch) = text_channel {
        if ch.kind != serenity::ChannelType::Text {
            return Err("Please specify a text channel for transcripts".into());
        }
        ch.id
    } else {
        // Use current channel
        ctx.channel_id()
    };

    ctx.defer().await?;

    // Save settings to database
    let new_settings = NewVoiceTranscriptSettings {
        guild_id: guild_id.to_string(),
        voice_channel_id: voice_channel_id.to_string(),
        text_channel_id: transcript_channel.to_string(),
        languages: lang_list.clone(),
    };

    VoiceTranscriptRepo::upsert(pool, new_settings).await?;

    // Create threads for each language
    let mut thread_names = Vec::new();
    for lang in &lang_list {
        let lang_name = Language::from_code(lang)
            .map(|l| l.name())
            .unwrap_or(lang.as_str());
        let thread_name = format!("Voice Translation - {}", lang_name);

        // Create a thread in the text channel
        let thread_builder = serenity::CreateThread::new(thread_name.clone())
            .kind(serenity::ChannelType::PublicThread)
            .auto_archive_duration(serenity::AutoArchiveDuration::OneDay);

        match transcript_channel
            .create_thread(ctx.http(), thread_builder)
            .await
        {
            Ok(thread) => {
                // Store thread ID in database
                VoiceTranscriptRepo::set_thread_id(
                    pool,
                    &guild_id.to_string(),
                    &voice_channel_id.to_string(),
                    lang,
                    &thread.id.to_string(),
                )
                .await
                .ok();

                thread_names.push(format!("<#{}> ({})", thread.id, lang.to_uppercase()));

                // Send initial message to thread
                let welcome_msg = format!(
                    "This thread will receive real-time voice transcripts translated to **{}**.\n\nTranscripts from <#{}> will appear here.",
                    lang_name, voice_channel_id
                );
                let _ = thread
                    .id
                    .send_message(ctx.http(), serenity::CreateMessage::new().content(welcome_msg))
                    .await;
            }
            Err(e) => {
                error!(error = %e, lang = %lang, "Failed to create transcript thread");
                thread_names.push(format!("{} (failed to create)", lang.to_uppercase()));
            }
        }
    }

    let lang_display = lang_list
        .iter()
        .map(|l| l.to_uppercase())
        .collect::<Vec<_>>()
        .join(", ");

    let threads_display = thread_names.join("\n");

    let embed = serenity::CreateEmbed::default()
        .title("Voice Transcripts Enabled")
        .description(format!(
            "Transcripts from <#{}> will be posted to language-specific threads.",
            voice_channel_id
        ))
        .field("Languages", &lang_display, true)
        .field("Voice Channel", format!("<#{}>", voice_channel_id), true)
        .field("Threads Created", &threads_display, false)
        .footer(serenity::CreateEmbedFooter::new(
            "Transcripts will appear in real-time as people speak",
        ))
        .color(0x57F287);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
