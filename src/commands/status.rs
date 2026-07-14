use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::data::{Context, Error};
use crate::db::queries;

// ─────────────────────────────────────────────────────────────────────────────
// /status — show the full server dashboard
// ─────────────────────────────────────────────────────────────────────────────

fn ch(id: Option<&String>) -> String {
    id.and_then(|s| s.parse::<u64>().ok())
      .map(|id| serenity::ChannelId::new(id).mention().to_string())
      .unwrap_or_else(|| "*not set*".to_string())
}

/// 📊 Show what channels are configured and what the bot is posting.
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    let cfg           = queries::get_or_create_guild(db, &guild_id).await?;
    let react_channels = queries::get_reaction_channels(db, &guild_id).await?;
    let react_users    = queries::get_reaction_users(db, &guild_id).await?;
    let emojis         = queries::get_emojis(db, &guild_id).await?;

    let ch_list = if react_channels.is_empty() {
        "*none*".to_string()
    } else {
        react_channels.iter().filter_map(|id| id.parse::<u64>().ok())
            .map(|id| serenity::ChannelId::new(id).mention().to_string())
            .collect::<Vec<_>>().join(", ")
    };

    let user_list = if react_users.is_empty() {
        "*none*".to_string()
    } else {
        react_users.iter().filter_map(|id| id.parse::<u64>().ok())
            .map(|id| serenity::UserId::new(id).mention().to_string())
            .collect::<Vec<_>>().join(", ")
    };

    let emoji_list = if emojis.is_empty() {
        "👍 *(default)*".to_string()
    } else {
        emojis.join(" ")
    };

    let react_status = if cfg.auto_react_enabled { "🟢 On" } else { "🔴 Off" };

    let interval_secs = cfg.posting_interval_secs;
    let interval_label = if interval_secs < 120 {
        format!("{}s", interval_secs)
    } else {
        format!("{:.0}m {}s", (interval_secs / 60) as f64, interval_secs % 60)
    };

    let embed = serenity::CreateEmbed::new()
        .title("📊 Bot Status")
        .color(0x5865F2)
        // ── Content channels ─────────────────────────────────────────────────
        .field("🖼️ Memes",           ch(cfg.meme_channel_id.as_ref()),          true)
        .field("📰 Gaming News",      ch(cfg.news_channel_id.as_ref()),          true)
        .field("🎁 Free Games",       ch(cfg.free_games_channel_id.as_ref()),    true)
        .field("🔞 NSFW",             ch(cfg.nsfw_channel_id.as_ref()),          true)
        .field("🎌 JAV Videos",       ch(cfg.jav_channel_id.as_ref()),           true)
        .field("🔥 Porn Videos",      ch(cfg.porn_video_channel_id.as_ref()),    true)
        .field("🌶️ OK.XXX",           ch(cfg.okxxx_channel_id.as_ref()),         true)
        .field("⚔️ Clash of Clans",   ch(cfg.coc_channel_id.as_ref()),          true)
        .field("🌍 X Global",         ch(cfg.twitter_global_channel_id.as_ref()),true)
        .field("🌏 X Asia",           ch(cfg.twitter_asia_channel_id.as_ref()),  true)
        // ── Extra NSFW splits (collapsed) ────────────────────────────────────
        .field("🔞 Rule34",           ch(cfg.rule34_channel_id.as_ref()),        true)
        .field("🔞 Hentai",           ch(cfg.hentai_channel_id.as_ref()),        true)
        .field("🧠 Brainrot",         ch(cfg.brainrot_channel_id.as_ref()),      true)
        // ── Settings ─────────────────────────────────────────────────────────
        .field("⏱️ Meme Interval",    interval_label,                            true)
        .field("⚡ Auto-React",        react_status,                             true)
        .field("😄 Emojis",           emoji_list,                               true)
        .field("📢 React Channels",   ch_list,                                   false)
        .field("👤 React Users",      user_list,                                 false)
        .footer(serenity::CreateEmbedFooter::new(
            "Memes: configurable (default 60s) • News: 5min • Free Games: 15min • JAV: 15min • Porn: 20min • OK.XXX: 25min • Clash of Clans: 10min | /setup to change channels • /post to post now"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
