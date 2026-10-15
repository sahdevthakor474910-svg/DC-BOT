use poise::serenity_prelude as serenity;
use crate::data::{Context, Error};
use crate::dmc::{calculator, gemini};

/// 🎮 Calculate boss hunt kill time, DPS, and score from a DMC: Peak of Combat screenshot
#[poise::command(
    slash_command,
    guild_only,
    description_localized("en-US", "Calculate boss hunt kill time & score from a DMC:PoC screenshot")
)]
pub async fn dmc(
    ctx: Context<'_>,
    #[description = "DMC battle results or leaderboard screenshot"] image: serenity::Attachment,
) -> Result<(), Error> {
    ctx.defer().await?;

    let api_key = &ctx.data().config.gemini_api_key;
    if api_key.trim().is_empty() {
        ctx.say("⚠️ **DMC Calculator Error:** `GEMINI_API_KEY` is not set in the bot's environment variables. Please add your free Google Gemini API key to your Render dashboard to enable boss hunt calculations!").await?;
        return Ok(());
    }

    // Verify attachment is an image
    let is_image = matches!(
        image.content_type.as_deref(),
        Some("image/png")
            | Some("image/jpeg")
            | Some("image/jpg")
            | Some("image/gif")
            | Some("image/webp")
    ) || {
        let name = image.filename.to_lowercase();
        name.ends_with(".png")
            || name.ends_with(".jpg")
            || name.ends_with(".jpeg")
            || name.ends_with(".webp")
    };

    if !is_image {
        ctx.say("❌ Please attach a valid screenshot image file (.png, .jpg, .webp).").await?;
        return Ok(());
    }

    match gemini::analyze_screenshot(&ctx.data().http_client, api_key, &image.url).await {
        Ok(screenshot_data) => {
            let reply = calculator::build_discord_message(&screenshot_data);
            ctx.say(reply).await?;
        }
        Err(e) => {
            tracing::warn!("DMC slash command analysis failed: {:#}", e);
            ctx.say("⚠️ Could not detect DMC battle scores in this screenshot. Please ensure the post-battle Results screen (showing **DMG PTS**, **Reward PTS**, **Boss PTS**) or Leaderboard is clear and legible.").await?;
        }
    }

    Ok(())
}
