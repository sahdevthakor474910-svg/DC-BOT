use std::sync::Arc;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::data::{Context, Error};
use crate::db::queries;
use crate::reddit;
use crate::news;
use crate::freegames;
use crate::jav;
use crate::porn;

// ────────────────────────────────────────────────────────────────────────────
// /admin parent (admin-only)
// ────────────────────────────────────────────────────────────────────────────

/// Admin configuration commands. Requires Manage Server.
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "MANAGE_GUILD",
    subcommands(
        "set_meme_channel",
        "set_brainrot_channel",
        "set_shitposting_channel",
        "set_instagram_channel",
        "set_news_channel",
        "set_free_games_channel",
        "set_nsfw_channel",
        "set_rule34_channel",
        "set_porn_channel",
        "set_hentai_channel",
        "set_jav_channel",
        "set_porn_video_channel",
        "toggle_auto_react",
        "add_reaction_user",
        "remove_reaction_user",
        "status",
        "force_refresh"
    )
)]
pub async fn admin(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// ── Channel setters ──────────────────────────────────────────────────────────

/// Set the #memes channel for general Reddit memes.
#[poise::command(slash_command, guild_only, rename = "set-meme-channel")]
pub async fn set_meme_channel(
    ctx: Context<'_>,
    #[description = "Channel to post r/memes & r/dankmemes"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_meme_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Meme channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set the #brainrot channel (r/brainrot content).
#[poise::command(slash_command, guild_only, rename = "set-brainrot-channel")]
pub async fn set_brainrot_channel(
    ctx: Context<'_>,
    #[description = "Channel to post r/brainrot content"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_brainrot_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Brainrot channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set the #shitposting channel (r/shitposting & r/whenthe).
#[poise::command(slash_command, guild_only, rename = "set-shitposting-channel")]
pub async fn set_shitposting_channel(
    ctx: Context<'_>,
    #[description = "Channel to post r/shitposting & r/whenthe"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_shitposting_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Shitposting channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set the #instagram-memes channel (r/196 content).
#[poise::command(slash_command, guild_only, rename = "set-instagram-channel")]
pub async fn set_instagram_channel(
    ctx: Context<'_>,
    #[description = "Channel to post r/196 content"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_instagram_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Instagram-memes channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set the #gaming-news channel.
#[poise::command(slash_command, guild_only, rename = "set-news-channel")]
pub async fn set_news_channel(
    ctx: Context<'_>,
    #[description = "Channel to post gaming news"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_news_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Gaming-news channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set the #free-games channel.
#[poise::command(slash_command, guild_only, rename = "set-free-games-channel")]
pub async fn set_free_games_channel(
    ctx: Context<'_>,
    #[description = "Channel to post free game alerts"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_free_games_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Free-games channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set the 🔞 NSFW channel for adult content. Channel must be Age-Restricted in Discord.
#[poise::command(slash_command, guild_only, rename = "set-nsfw-channel")]
pub async fn set_nsfw_channel(
    ctx: Context<'_>,
    #[description = "Age-restricted channel to post adult content"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    // Enforce that the channel is actually marked as NSFW in Discord
    if !channel.nsfw {
        ctx.say("❌ That channel is **not** marked as Age-Restricted in Discord!\n\
            Go to **Channel Settings → Overview → Age-Restricted Channel** and enable it first.").await?;
        return Ok(());
    }
    queries::set_nsfw_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ NSFW channel → {} (r/nsfw, r/gonewild, r/rule34, r/hentai, r/porn)", channel.id.mention())).await?;
    Ok(())
}

/// Set a dedicated channel for r/rule34 content. Must be Age-Restricted in Discord.
#[poise::command(slash_command, guild_only, rename = "set-rule34-channel")]
pub async fn set_rule34_channel(
    ctx: Context<'_>,
    #[description = "Age-restricted channel to post r/rule34 content"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    if !channel.nsfw {
        ctx.say("❌ That channel is **not** marked as Age-Restricted in Discord!\n\
            Go to **Channel Settings → Overview → Age-Restricted Channel** and enable it first.").await?;
        return Ok(());
    }
    queries::set_rule34_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Rule34 channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set a dedicated channel for r/porn content. Must be Age-Restricted in Discord.
#[poise::command(slash_command, guild_only, rename = "set-porn-channel")]
pub async fn set_porn_channel(
    ctx: Context<'_>,
    #[description = "Age-restricted channel to post r/porn content"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    if !channel.nsfw {
        ctx.say("❌ That channel is **not** marked as Age-Restricted in Discord!\n\
            Go to **Channel Settings → Overview → Age-Restricted Channel** and enable it first.").await?;
        return Ok(());
    }
    queries::set_porn_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Porn channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set a dedicated channel for r/hentai content. Must be Age-Restricted in Discord.
#[poise::command(slash_command, guild_only, rename = "set-hentai-channel")]
pub async fn set_hentai_channel(
    ctx: Context<'_>,
    #[description = "Age-restricted channel to post r/hentai content"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    if !channel.nsfw {
        ctx.say("❌ That channel is **not** marked as Age-Restricted in Discord!\n\
            Go to **Channel Settings → Overview → Age-Restricted Channel** and enable it first.").await?;
        return Ok(());
    }
    queries::set_hentai_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ Hentai channel → {}", channel.id.mention())).await?;
    Ok(())
}

/// Set a dedicated 🎌 JAV channel (latest releases + popular titles). Must be Age-Restricted.
#[poise::command(slash_command, guild_only, rename = "set-jav-channel")]
pub async fn set_jav_channel(
    ctx: Context<'_>,
    #[description = "Age-restricted channel to post JAV titles"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    if !channel.nsfw {
        ctx.say("❌ That channel is **not** marked as Age-Restricted in Discord!\n\
            Go to **Channel Settings → Overview → Age-Restricted Channel** and enable it first.").await?;
        return Ok(());
    }
    queries::set_jav_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!("✅ JAV channel → {} (posts from r/jav + r/javonline every 30 min)", channel.id.mention())).await?;
    Ok(())
}

/// Set a 🔞 Porn Video channel (RedTube: NaughtyAmerica, Brazzers, MILF, etc.)
#[poise::command(slash_command, guild_only, rename = "set-porn-video-channel")]
pub async fn set_porn_video_channel(
    ctx: Context<'_>,
    #[description = "Age-restricted channel for real porn videos from RedTube"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    if !channel.nsfw {
        ctx.say("❌ That channel is **not** marked as Age-Restricted in Discord!\n\
            Go to **Channel Settings → Overview → Age-Restricted Channel** and enable it first.").await?;
        return Ok(());
    }
    queries::set_porn_video_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;
    ctx.say(format!(
        "✅ Porn Video channel → {}\n🔥 Posts studio videos (NaughtyAmerica, Brazzers, MILF etc.) every 45 min from RedTube!",
        channel.id.mention()
    )).await?;
    Ok(())
}

// ── Auto-react toggle ────────────────────────────────────────────────────────

/// Toggle automatic reactions on or off for this server.
#[poise::command(slash_command, guild_only, rename = "toggle-auto-react")]
pub async fn toggle_auto_react(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let cfg = queries::get_or_create_guild(&ctx.data().db, &guild_id).await?;
    let new_state = !cfg.auto_react_enabled;
    queries::set_auto_react_enabled(&ctx.data().db, &guild_id, new_state).await?;
    let label = if new_state { "🟢 enabled" } else { "🔴 disabled" };
    ctx.say(format!("Auto-react is now **{}**.", label)).await?;
    Ok(())
}

// ── Reaction user management ─────────────────────────────────────────────────

/// Add a user whose messages will always receive auto-reactions.
#[poise::command(slash_command, guild_only, rename = "add-reaction-user")]
pub async fn add_reaction_user(
    ctx: Context<'_>,
    #[description = "User to auto-react to"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::add_reaction_user(&ctx.data().db, &guild_id, &user.id.to_string()).await?;
    ctx.say(format!("✅ {} added to auto-react users.", user.mention())).await?;
    Ok(())
}

/// Remove a user from the auto-react list.
#[poise::command(slash_command, guild_only, rename = "remove-reaction-user")]
pub async fn remove_reaction_user(
    ctx: Context<'_>,
    #[description = "User to remove from auto-react"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::remove_reaction_user(&ctx.data().db, &guild_id, &user.id.to_string()).await?;
    ctx.say(format!("✅ {} removed from auto-react users.", user.mention())).await?;
    Ok(())
}

// ── Status dashboard ─────────────────────────────────────────────────────────

fn ch_mention(id: Option<&String>) -> String {
    id.and_then(|s| s.parse::<u64>().ok())
      .map(|id| serenity::ChannelId::new(id).mention().to_string())
      .unwrap_or_else(|| "*not set*".to_string())
}

/// Show a full configuration status dashboard for this server.
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;
    let cfg = queries::get_or_create_guild(db, &guild_id).await?;
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

    let embed = serenity::CreateEmbed::new()
        .title("🤖 Bot Status Dashboard")
        .color(0x5865F2)
        .field("🖼️ Memes",         ch_mention(cfg.meme_channel_id.as_ref()),        true)
        .field("🧠 Brainrot",       ch_mention(cfg.brainrot_channel_id.as_ref()),    true)
        .field("💩 Shitposting",    ch_mention(cfg.shitposting_channel_id.as_ref()), true)
        .field("📸 Instagram Memes",ch_mention(cfg.instagram_channel_id.as_ref()),   true)
        .field("🎮 Gaming News",    ch_mention(cfg.news_channel_id.as_ref()),         true)
        .field("🎁 Free Games",     ch_mention(cfg.free_games_channel_id.as_ref()),  true)
        .field("🔞 NSFW (Other)",   ch_mention(cfg.nsfw_channel_id.as_ref()),        true)
        .field("🔞 Rule34",         ch_mention(cfg.rule34_channel_id.as_ref()),      true)
        .field("🔞 Porn",           ch_mention(cfg.porn_channel_id.as_ref()),        true)
        .field("🔞 Hentai",         ch_mention(cfg.hentai_channel_id.as_ref()),           true)
        .field("🎌 JAV Videos",     ch_mention(cfg.jav_channel_id.as_ref()),               true)
        .field("🔥 Porn Videos",    ch_mention(cfg.porn_video_channel_id.as_ref()),        true)
        .field("⚡ Auto-React",     react_status,                                          true)
        .field("😄 Emojis",         emoji_list,                                            false)
        .field("📢 React Channels", ch_list,                                               false)
        .field("👤 React Users",    user_list,                                             false)
        .footer(serenity::CreateEmbedFooter::new(
            "Memes: 5min • News: 15min • Free Games: 30min • JAV: 30min • Porn Videos: 45min"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

// ── Force refresh ─────────────────────────────────────────────────────────────

/// Immediately trigger memes, gaming news, and free-game tasks. May take a moment.
#[poise::command(slash_command, guild_only, rename = "force-refresh")]
pub async fn force_refresh(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data().clone();
    let http: Arc<serenity::Http> = Arc::clone(&ctx.serenity_context().http);

    // Run all tasks concurrently
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

    ctx.say(format!(
        "✅ Force refresh complete!\n\
        📸 Memes posted: **{}**\n\
        📰 News articles posted: **{}**\n\
        🎁 Free game alerts posted: **{}**\n\
        🎌 JAV titles posted: **{}**\n\
        🔞 Porn videos posted: **{}**",
        meme_n, news_n, fg_n, jav_n, porn_n
    ))
    .await?;

    Ok(())
}
