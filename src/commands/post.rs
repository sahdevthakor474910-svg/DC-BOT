use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::data::{Context, Error};
use crate::reddit;
use crate::news;
use crate::freegames;
use crate::jav;
use crate::porn;

// ─────────────────────────────────────────────────────────────────────────────
// /post — instantly post all content right now
// ─────────────────────────────────────────────────────────────────────────────

/// ⚡ Post memes, news, free games & NSFW content right now — don't wait for the timer!
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check"
)]
pub async fn post(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data().clone();
    let http: Arc<serenity::Http> = Arc::clone(&ctx.serenity_context().http);

    let (meme_res, news_res, fg_res, jav_res, porn_res) = tokio::join!(
        reddit::task::run_once(&data, &http),
        news::task::run_once(&data, &http),
        freegames::task::run_once(&data, &http),
        jav::task::run_once(&data, &http),
        porn::task::run_once(&data, &http),
    );

    let meme_n = meme_res.unwrap_or_else(|e| { tracing::error!("Meme refresh: {:#}", e); 0 });
    let news_n = news_res.unwrap_or_else(|e| { tracing::error!("News refresh: {:#}", e); 0 });
    let fg_n   = fg_res.unwrap_or_else(|e|   { tracing::error!("FG refresh: {:#}", e);   0 });
    let jav_n  = jav_res.unwrap_or_else(|e|  { tracing::error!("JAV refresh: {:#}", e);  0 });
    let porn_n = porn_res.unwrap_or_else(|e| { tracing::error!("Porn refresh: {:#}", e); 0 });

    let total = meme_n + news_n + fg_n + jav_n + porn_n;

    if total == 0 {
        ctx.say(
            "📭 Nothing new to post — everything is already up to date!\n\n\
            *The bot tracks what it has already posted to avoid duplicates. \
            New content will appear automatically on the next cycle.*"
        ).await?;
        return Ok(());
    }

    let embed = serenity::CreateEmbed::new()
        .title("⚡ Posted Right Now!")
        .color(0x57F287) // Discord green
        .field("🖼️ Memes",         meme_n.to_string(), true)
        .field("📰 Gaming News",   news_n.to_string(), true)
        .field("🎁 Free Games",    fg_n.to_string(),   true)
        .field("🎌 JAV Videos",    jav_n.to_string(),  true)
        .field("🔥 Porn Videos",   porn_n.to_string(), true)
        .field("📬 Total",         total.to_string(),  true)
        .footer(serenity::CreateEmbedFooter::new(
            "New content posts automatically — use /post anytime to force it!"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
