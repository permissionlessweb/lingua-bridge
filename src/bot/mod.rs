pub mod commands;
pub mod handler;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::translation::TranslationClient;
use crate::voice::{spawn_voice_bridge, VoiceClientConfig, VoiceManager};
use crate::web::broadcast::BroadcastManager;
use poise::serenity_prelude::{self as serenity, FullEvent, GatewayIntents};
use songbird::SerenityInit;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// Shared data accessible in all commands
#[derive(Debug)]
pub struct Data {
    pub pool: DbPool,
    pub translator: Arc<TranslationClient>,
    pub broadcast: Arc<BroadcastManager>,
    pub voice: Option<Arc<VoiceManager>>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Event handler for Discord events
async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot } => {
            info!(
                "Bot is ready! Logged in as {}",
                data_about_bot.user.name
            );
        }
        FullEvent::Message { new_message } => {
            handler::handle_message(
                ctx,
                new_message,
                &data.pool,
                &data.translator,
                &data.broadcast,
            )
            .await;
        }
        FullEvent::GuildCreate { guild, is_new: _ } => {
            handler::handle_guild_create(guild, &data.pool).await;
        }
        FullEvent::GuildDelete { incomplete, full: _ } => {
            handler::handle_guild_delete(incomplete.id).await;
        }
        _ => {}
    }
    Ok(())
}

/// Create and configure the Discord bot framework
pub async fn create_framework(
    pool: DbPool,
    translator: Arc<TranslationClient>,
    broadcast: Arc<BroadcastManager>,
    voice: Option<Arc<VoiceManager>>,
) -> Result<poise::Framework<Data, Error>, Error> {
    let _config = AppConfig::get();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |error| {
                Box::pin(async move {
                    match error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            error!("Command error: {}", error);
                            let _ = ctx.say(format!("An error occurred: {}", error)).await;
                        }
                        poise::FrameworkError::Setup { error, .. } => {
                            error!("Setup error: {}", error);
                        }
                        err => {
                            error!("Framework error: {:?}", err);
                        }
                    }
                })
            },
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!lb ".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("Registered {} slash commands globally", framework.options().commands.len());
                Ok(Data {
                    pool,
                    translator,
                    broadcast,
                    voice,
                })
            })
        })
        .build();

    Ok(framework)
}

/// Start the Discord bot with a token from the SecretStore.
///
/// This is the primary entry point used after admin provisioning.
pub async fn start_bot_with_token(
    pool: DbPool,
    translator: Arc<TranslationClient>,
    broadcast: Arc<BroadcastManager>,
    token: &str,
) -> Result<(), Error> {
    if token.is_empty() {
        return Err("Discord token is empty".into());
    }

    let config = AppConfig::get();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_VOICE_STATES;

    // Create Songbird voice manager
    let songbird = songbird::Songbird::serenity();

    // Create voice client config from app config
    let voice_client_config = VoiceClientConfig {
        url: config.voice.url.clone(),
        reconnect_delay: Duration::from_secs(2),
        max_reconnect_attempts: 10,
        request_timeout: Duration::from_secs(30),
        ping_interval: Duration::from_secs(30),
    };

    // Create voice manager
    let voice_manager = Arc::new(VoiceManager::new(songbird.clone(), voice_client_config));

    // Spawn voice bridge to forward results to web clients
    let voice_rx = voice_manager.subscribe_results();
    let _bridge_handle = spawn_voice_bridge(voice_rx, broadcast.clone());
    info!("Voice bridge started - forwarding transcriptions to web clients");

    let framework = create_framework(pool, translator, broadcast, Some(voice_manager)).await?;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird_with(songbird.clone())
        .await?;

    info!("Starting Discord bot with voice support...");
    client.start().await?;

    Ok(())
}
