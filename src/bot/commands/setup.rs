use crate::bot::Data;
use crate::db::{GuildRepo, NewGuild};
use crate::translation::Language;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Setup LinguaBridge for your server
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "ADMINISTRATOR",
    subcommands("setup_init", "setup_channel", "setup_languages", "setup_status")
)]
pub async fn setup(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Initialize LinguaBridge for this server
#[poise::command(slash_command, guild_only, rename = "init")]
pub async fn setup_init(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, guild_name) = {
        let guild = ctx.guild().ok_or("Must be used in a guild")?;
        (guild.id.to_string(), guild.name.clone())
    };

    let new_guild = NewGuild {
        guild_id: guild_id.clone(),
        name: guild_name.clone(),
    };

    GuildRepo::upsert(&ctx.data().pool, new_guild).await?;

    ctx.say(format!(
        "LinguaBridge initialized for **{}**!\n\n\
        Use `/setup channel` to enable translation in specific channels.\n\
        Use `/setup languages` to configure target languages.",
        guild_name
    ))
    .await?;

    Ok(())
}

/// Enable or disable translation in a channel
#[poise::command(slash_command, guild_only, rename = "channel")]
pub async fn setup_channel(
    ctx: Context<'_>,
    #[description = "Channel to configure"] channel: serenity::GuildChannel,
    #[description = "Enable translation"] enable: bool,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.to_string();
    let channel_id = channel.id.to_string();

    // Ensure guild exists
    if GuildRepo::get_by_guild_id(&ctx.data().pool, &guild_id)
        .await?
        .is_none()
    {
        ctx.say("Please run `/setup init` first to initialize LinguaBridge.").await?;
        return Ok(());
    }

    if enable {
        GuildRepo::enable_channel(&ctx.data().pool, &guild_id, &channel_id).await?;
        ctx.say(format!("Translation enabled in <#{}>", channel.id)).await?;
    } else {
        GuildRepo::disable_channel(&ctx.data().pool, &guild_id, &channel_id).await?;
        ctx.say(format!("Translation disabled in <#{}>", channel.id)).await?;
    }

    Ok(())
}

/// Configure target languages for translation
#[poise::command(slash_command, guild_only, rename = "languages")]
pub async fn setup_languages(
    ctx: Context<'_>,
    #[description = "Languages (comma-separated, e.g., 'en,es,fr')"] languages: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.to_string();

    // Ensure guild exists
    let settings = GuildRepo::get_settings(&ctx.data().pool, &guild_id)
        .await?
        .ok_or("Please run `/setup init` first")?;

    // Parse and validate languages
    let langs: Vec<String> = languages
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    // Validate against supported languages
    let mut valid_langs = Vec::new();
    let mut invalid_langs = Vec::new();

    for lang in &langs {
        if Language::from_code(lang).is_some() {
            valid_langs.push(lang.clone());
        } else {
            invalid_langs.push(lang.clone());
        }
    }

    // Check tier limits
    let max_langs = settings.subscription_tier.max_languages();
    if valid_langs.len() > max_langs {
        ctx.say(format!(
            "Your subscription tier ({}) allows up to {} languages. \
            Upgrade to add more!",
            settings.subscription_tier, max_langs
        ))
        .await?;
        return Ok(());
    }

    GuildRepo::set_target_languages(&ctx.data().pool, &guild_id, &valid_langs).await?;

    let mut response = format!(
        "Target languages set: **{}**",
        valid_langs.join(", ")
    );

    if !invalid_langs.is_empty() {
        response.push_str(&format!(
            "\n\nUnknown languages ignored: {}",
            invalid_langs.join(", ")
        ));
    }

    ctx.say(response).await?;
    Ok(())
}

/// Show current LinguaBridge configuration
#[poise::command(slash_command, guild_only, rename = "status")]
pub async fn setup_status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.to_string();

    let settings = match GuildRepo::get_settings(&ctx.data().pool, &guild_id).await? {
        Some(s) => s,
        None => {
            ctx.say("LinguaBridge is not configured for this server. Run `/setup init` to get started.")
                .await?;
            return Ok(());
        }
    };

    let channels_str = if settings.enabled_channels.is_empty() {
        "None".to_string()
    } else {
        settings
            .enabled_channels
            .iter()
            .map(|c| format!("<#{}>", c))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let embed = serenity::CreateEmbed::default()
        .title("LinguaBridge Configuration")
        .field("Default Language", &settings.default_language, true)
        .field("Subscription", settings.subscription_tier.as_str(), true)
        .field(
            "Target Languages",
            settings.target_languages.join(", "),
            false,
        )
        .field("Enabled Channels", channels_str, false)
        .color(0x5865F2);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
