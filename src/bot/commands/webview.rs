use crate::bot::Data;
use crate::config::AppConfig;
use crate::db::{GuildRepo, NewWebSession, WebSessionRepo};
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Generate a link to the web translation view
#[poise::command(slash_command, guild_only)]
pub async fn webview(
    ctx: Context<'_>,
    #[description = "Channel to view (current channel if not specified)"]
    channel: Option<serenity::GuildChannel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.to_string();
    let user_id = ctx.author().id.to_string();
    let channel_id = channel
        .as_ref()
        .map(|c| c.id.to_string())
        .or_else(|| Some(ctx.channel_id().to_string()));

    // Check if guild is configured
    let settings = GuildRepo::get_settings(&ctx.data().pool, &guild_id)
        .await?
        .ok_or("This server hasn't been set up yet. Ask an admin to run `/setup init`.")?;

    // Check subscription for web view access
    if !settings.subscription_tier.has_web_view() {
        ctx.say(
            "Web view is available for Basic and Pro subscribers.\n\
            Contact your server admin about upgrading!",
        )
        .await?;
        return Ok(());
    }

    // Create session
    let session = WebSessionRepo::create(
        &ctx.data().pool,
        NewWebSession {
            user_id: user_id.clone(),
            guild_id: guild_id.clone(),
            channel_id: channel_id.clone(),
        },
        AppConfig::get().web.session_expiry_hours,
    )
    .await?;

    let config = AppConfig::get();
    let web_url = format!(
        "{}/view/{}",
        config.web.public_url.trim_end_matches('/'),
        session.session_id
    );

    let channel_mention = channel_id
        .as_ref()
        .map(|id| format!("<#{}>", id))
        .unwrap_or_else(|| "all channels".to_string());

    let embed = serenity::CreateEmbed::default()
        .title("Web Translation View")
        .description(format!(
            "View live translations for {} in your browser.\n\n\
            **[Click here to open]({})** \n\n\
            This link expires in {} hours.",
            channel_mention,
            web_url,
            config.web.session_expiry_hours
        ))
        .field("Session ID", &session.session_id[..8], true)
        .footer(serenity::CreateEmbedFooter::new(
            "Keep this link private - it's tied to your account",
        ))
        .color(0x5865F2);

    // Send as ephemeral to keep the link private
    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
        .await?;

    Ok(())
}
