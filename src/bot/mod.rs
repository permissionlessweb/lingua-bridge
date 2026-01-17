pub mod commands;
pub mod handler;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::translation::TranslationClient;
use crate::web::broadcast::BroadcastManager;
use poise::serenity_prelude::{self as serenity, FullEvent, GatewayIntents};
use std::sync::Arc;
use tracing::{error, info};

/// Shared data accessible in all commands
#[derive(Debug)]
pub struct Data {
    pub pool: DbPool,
    pub translator: Arc<TranslationClient>,
    pub broadcast: Arc<BroadcastManager>,
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
) -> Result<poise::Framework<Data, Error>, Error> {
    let config = AppConfig::get();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS;

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

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS;

    let framework = create_framework(pool, translator, broadcast).await?;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    info!("Starting Discord bot...");
    client.start().await?;

    Ok(())
}
