use crate::bot::Data;
use crate::translation::Language;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Translate text to a specific language
#[poise::command(slash_command, guild_only)]
pub async fn translate(
    ctx: Context<'_>,
    #[description = "Text to translate"] text: String,
    #[description = "Target language code (e.g., 'es', 'fr', 'ja')"] target: String,
    #[description = "Source language (auto-detect if not specified)"] source: Option<String>,
) -> Result<(), Error> {
    // Validate target language
    let target_lang = Language::from_code(&target)
        .ok_or_else(|| format!("Unknown language: {}. Use ISO 639-1 codes like 'en', 'es', 'fr'.", target))?;

    // Validate source language if provided
    if let Some(ref src) = source {
        if Language::from_code(src).is_none() {
            return Err(format!("Unknown source language: {}", src).into());
        }
    }

    // Defer response since translation may take time
    ctx.defer().await?;

    let result = if let Some(src_lang) = source {
        ctx.data()
            .translator
            .translate(&text, &src_lang, target_lang.code())
            .await?
    } else {
        ctx.data()
            .translator
            .translate_auto(&text, target_lang.code())
            .await?
    };

    let embed = serenity::CreateEmbed::default()
        .title("Translation")
        .field("Original", &result.original_text, false)
        .field(target_lang.name(), &result.translated_text, false)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "{} → {} {}",
            result.source_lang.to_uppercase(),
            result.target_lang.to_uppercase(),
            if result.cached { "(cached)" } else { "" }
        )))
        .color(0x5865F2);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// List all supported languages
#[poise::command(slash_command)]
pub async fn languages(ctx: Context<'_>) -> Result<(), Error> {
    let langs: Vec<String> = Language::all()
        .iter()
        .map(|l| format!("`{}` - {}", l.code(), l.name()))
        .collect();

    // Split into chunks for Discord message limits
    let chunks: Vec<&[String]> = langs.chunks(15).collect();

    let embed = serenity::CreateEmbed::default()
        .title("Supported Languages")
        .description(format!(
            "LinguaBridge supports {} languages:\n\n{}",
            Language::all().len(),
            chunks[0].join("\n")
        ))
        .footer(serenity::CreateEmbedFooter::new(
            "Use language codes in commands (e.g., /translate text:Hello target:es)",
        ))
        .color(0x5865F2);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
