use poise::serenity_prelude::Mentionable;

use crate::data::{Context, Error};
use crate::db::queries;
use crate::reddit::client::SUBREDDITS;
use crate::reddit::task;


// ────────────────────────────────────────────────────────────────────────────
// /memes parent command
// ────────────────────────────────────────────────────────────────────────────

/// Meme task management commands.
#[poise::command(
    slash_command,
    guild_only,
    subcommands("status", "fetch_now")
)]
pub async fn memes(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /memes status
// ────────────────────────────────────────────────────────────────────────────

/// Show meme task status and statistics for this server.
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    let cfg = queries::get_or_create_guild(db, &guild_id).await?;

    let meme_ch = cfg
        .meme_channel_id
        .as_deref()
        .map(|id| {
            let id_u64: u64 = id.parse().unwrap_or(0);
            poise::serenity_prelude::ChannelId::new(id_u64)
                .mention()
                .to_string()
        })
        .unwrap_or_else(|| "*not configured*".to_string());

    let subreddit_list = SUBREDDITS
        .iter()
        .map(|s| format!("r/{}", s))
        .collect::<Vec<_>>()
        .join(", ");

    ctx.say(format!(
        "## 📊 Meme Task Status\n\
        **Meme Channel:** {meme_ch}\n\
        **Fetch Interval:** {} seconds\n\
        **Subreddits:** {subreddit_list}\n\
        **Task Status:** 🟢 Running",
        cfg.posting_interval_secs,
    ))
    .await?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /memes fetch-now
// ────────────────────────────────────────────────────────────────────────────

/// Immediately trigger a meme fetch and post cycle (admin only).
#[poise::command(
    slash_command,
    guild_only,
    rename = "fetch-now",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn fetch_now(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().unwrap().to_string();
    let cfg = queries::get_or_create_guild(&ctx.data().db, &guild_id).await?;

    if cfg.meme_channel_id.is_none() {
        ctx.say("❌ No meme channel configured. Use `/config meme-channel` first.")
            .await?;
        return Ok(());
    }

    // Spawn a one-shot fetch using the shared Data
    let data = ctx.data().clone();
    let http = ctx.serenity_context().http.clone();

    let handle = tokio::spawn(async move {
        task::run_once(&data, &http).await
    });

    match handle.await {
        Ok(Ok(posted)) => {
            ctx.say(format!("✅ Fetch complete! Posted **{}** new meme(s).", posted))
                .await?;
        }
        Ok(Err(e)) => {
            ctx.say(format!("❌ Fetch failed: {}", e)).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Task panicked: {}", e)).await?;
        }
    }

    Ok(())
}
