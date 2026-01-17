use crate::bot::Data;
use crate::db::UserPreferenceRepo;
use crate::translation::Language;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Set your preferred language for translations
#[poise::command(slash_command, guild_only)]
pub async fn mylang(
    ctx: Context<'_>,
    #[description = "Your preferred language code (e.g., 'en', 'es', 'fr')"] language: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.to_string();
    let user_id = ctx.author().id.to_string();

    // Validate language
    let lang = Language::from_code(&language).ok_or_else(|| {
        format!(
            "Unknown language: {}. Use ISO 639-1 codes like 'en', 'es', 'fr'.\n\
            Use `/languages` to see all supported languages.",
            language
        )
    })?;

    UserPreferenceRepo::set_language(&ctx.data().pool, &user_id, &guild_id, lang.code()).await?;

    ctx.say(format!(
        "Your preferred language has been set to **{}** ({}).\n\
        Translations will be delivered in this language when available.",
        lang.name(),
        lang.code()
    ))
    .await?;

    Ok(())
}

/// Check your current language preference
#[poise::command(slash_command, guild_only)]
pub async fn mypreferences(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.to_string();
    let user_id = ctx.author().id.to_string();

    let pref = UserPreferenceRepo::get(&ctx.data().pool, &user_id, &guild_id).await?;

    match pref {
        Some(p) => {
            let lang_name = Language::from_code(&p.preferred_language)
                .map(|l| l.name())
                .unwrap_or("Unknown");

            let embed = serenity::CreateEmbed::default()
                .title("Your LinguaBridge Preferences")
                .field(
                    "Preferred Language",
                    format!("{} (`{}`)", lang_name, p.preferred_language),
                    true,
                )
                .field(
                    "Auto-Translate",
                    if p.auto_translate { "Enabled" } else { "Disabled" },
                    true,
                )
                .color(0x5865F2);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        None => {
            ctx.say(
                "You haven't set any preferences yet.\n\
                Use `/mylang <code>` to set your preferred language.",
            )
            .await?;
        }
    }

    Ok(())
}
