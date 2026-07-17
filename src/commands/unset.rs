use crate::data::{Context, Error};
use crate::db::queries;

// ─────────────────────────────────────────────────────────────────────────────
// /unset — disable / clear a previously configured channel
// ─────────────────────────────────────────────────────────────────────────────

/// 🗑️ Disable a channel — stop the bot from posting there.
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check"
)]
pub async fn unset(
    ctx: Context<'_>,

    #[description = "Clear the 🖼️ Memes channel"]
    memes: Option<bool>,

    #[description = "Clear the 📰 Gaming News channel"]
    news: Option<bool>,

    #[description = "Clear the 🎁 Free Games channel"]
    free_games: Option<bool>,

    #[description = "Clear the 🔞 NSFW channel"]
    nsfw: Option<bool>,

    #[description = "Clear the 🎌 JAV channel"]
    jav: Option<bool>,

    #[description = "Clear the 🔥 Porn Videos channel"]
    porn_videos: Option<bool>,

    #[description = "Clear the 🌶️ OK.XXX channel"]
    okxxx: Option<bool>,

    #[description = "Clear the ⚔️ Clash of Clans channel"]
    coc: Option<bool>,

    #[description = "Clear the 📸 Hot Photos channel"]
    hot_photos: Option<bool>,

    #[description = "Clear the 🌍 X / Twitter Global channel"]
    twitter_global: Option<bool>,

    #[description = "Clear the 🌏 X / Twitter Asia channel"]
    twitter_asia: Option<bool>,

    #[description = "Clear the 🎮 DMC Boss Results channel"]
    dmc: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    let mut cleared: Vec<&str> = Vec::new();

    if memes.unwrap_or(false) {
        queries::set_meme_channel(db, &guild_id, None).await?;
        queries::set_brainrot_channel(db, &guild_id, None).await?;
        queries::set_shitposting_channel(db, &guild_id, None).await?;
        queries::set_instagram_channel(db, &guild_id, None).await?;
        cleared.push("🖼️ Memes");
    }

    if news.unwrap_or(false) {
        queries::set_news_channel(db, &guild_id, None).await?;
        cleared.push("📰 Gaming News");
    }

    if free_games.unwrap_or(false) {
        queries::set_free_games_channel(db, &guild_id, None).await?;
        cleared.push("🎁 Free Games");
    }

    if nsfw.unwrap_or(false) {
        queries::set_nsfw_channel(db, &guild_id, None).await?;
        queries::set_rule34_channel(db, &guild_id, None).await?;
        queries::set_hentai_channel(db, &guild_id, None).await?;
        cleared.push("🔞 NSFW");
    }

    if jav.unwrap_or(false) {
        queries::set_jav_channel(db, &guild_id, None).await?;
        cleared.push("🎌 JAV Videos");
    }

    if porn_videos.unwrap_or(false) {
        queries::set_porn_video_channel(db, &guild_id, None).await?;
        cleared.push("🔥 Porn Videos");
    }

    if okxxx.unwrap_or(false) {
        queries::set_okxxx_channel(db, &guild_id, None).await?;
        cleared.push("🌶️ OK.XXX");
    }

    if coc.unwrap_or(false) {
        queries::set_coc_channel(db, &guild_id, None).await?;
        cleared.push("⚔️ Clash of Clans");
    }

    if hot_photos.unwrap_or(false) {
        queries::set_porn_channel(db, &guild_id, None).await?;
        cleared.push("📸 Hot Photos");
    }

    if twitter_global.unwrap_or(false) {
        queries::set_twitter_global_channel(db, &guild_id, None).await?;
        cleared.push("🌍 X Global");
    }

    if twitter_asia.unwrap_or(false) {
        queries::set_twitter_asia_channel(db, &guild_id, None).await?;
        cleared.push("🌏 X Asia");
    }

    if dmc.unwrap_or(false) {
        queries::set_dmc_channel(db, &guild_id, None).await?;
        cleared.push("🎮 DMC Boss Results");
    }

    if cleared.is_empty() {
        ctx.say(
            "❌ You didn't select anything to unset!\n\
            Run `/unset` again and set any channel option to **True** to disable it.",
        )
        .await?;
        return Ok(());
    }

    let list = cleared
        .iter()
        .map(|ch| format!("• {}", ch))
        .collect::<Vec<_>>()
        .join("\n");

    ctx.say(format!(
        "✅ **Cleared the following channels** — the bot will stop posting there:\n\n{}\n\n\
        💡 Use `/setup` to re-enable any of them.",
        list
    ))
    .await?;

    Ok(())
}
