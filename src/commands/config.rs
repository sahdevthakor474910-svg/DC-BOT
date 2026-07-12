use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::data::{Context, Error};
use crate::db::queries;

// ────────────────────────────────────────────────────────────────────────────
// /config — parent command (admin-only)
// ────────────────────────────────────────────────────────────────────────────

/// Bot configuration commands (administrator only).
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check",
    subcommands(
        "meme_channel",
        "add_reaction_channel",
        "remove_reaction_channel",
        "add_user",
        "remove_user",
        "add_emoji",
        "remove_emoji",
        "interval",
        "show"
    )
)]
pub async fn config(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /config meme-channel
// ────────────────────────────────────────────────────────────────────────────

/// Set the channel where memes will be automatically posted.
#[poise::command(slash_command, guild_only, rename = "meme-channel")]
pub async fn meme_channel(
    ctx: Context<'_>,
    #[description = "Channel to post memes in"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();

    queries::set_meme_channel(&ctx.data().db, &guild_id, Some(channel.id.to_string().as_str())).await?;

    ctx.say(format!(
        "✅ Meme channel set to {}",
        channel.id.mention()
    ))
    .await?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /config add-reaction-channel / remove-reaction-channel
// ────────────────────────────────────────────────────────────────────────────

/// Add a channel to the auto-react list.
#[poise::command(slash_command, guild_only, rename = "add-reaction-channel")]
pub async fn add_reaction_channel(
    ctx: Context<'_>,
    #[description = "Channel to auto-react in"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();

    queries::add_reaction_channel(&ctx.data().db, &guild_id, &channel.id.to_string()).await?;

    ctx.say(format!(
        "✅ {} added to auto-react channels.",
        channel.id.mention()
    ))
    .await?;

    Ok(())
}

/// Remove a channel from the auto-react list.
#[poise::command(slash_command, guild_only, rename = "remove-reaction-channel")]
pub async fn remove_reaction_channel(
    ctx: Context<'_>,
    #[description = "Channel to remove from auto-react"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();

    queries::remove_reaction_channel(&ctx.data().db, &guild_id, &channel.id.to_string()).await?;

    ctx.say(format!(
        "✅ {} removed from auto-react channels.",
        channel.id.mention()
    ))
    .await?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /config add-user / remove-user
// ────────────────────────────────────────────────────────────────────────────

/// Add a user whose messages will always be auto-reacted to.
#[poise::command(slash_command, guild_only, rename = "add-user")]
pub async fn add_user(
    ctx: Context<'_>,
    #[description = "User to auto-react to"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();

    queries::add_reaction_user(&ctx.data().db, &guild_id, &user.id.to_string()).await?;

    ctx.say(format!("✅ {} added to auto-react users.", user.mention()))
        .await?;

    Ok(())
}

/// Remove a user from the auto-react list.
#[poise::command(slash_command, guild_only, rename = "remove-user")]
pub async fn remove_user(
    ctx: Context<'_>,
    #[description = "User to remove from auto-react"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();

    queries::remove_reaction_user(&ctx.data().db, &guild_id, &user.id.to_string()).await?;

    ctx.say(format!(
        "✅ {} removed from auto-react users.",
        user.mention()
    ))
    .await?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /config add-emoji / remove-emoji
// ────────────────────────────────────────────────────────────────────────────

/// Add an emoji to the reaction list (Unicode or custom: `<:name:id>`).
#[poise::command(slash_command, guild_only, rename = "add-emoji")]
pub async fn add_emoji(
    ctx: Context<'_>,
    #[description = "Emoji to add (e.g. 🔥 or <:name:id>)"] emoji: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let emoji = emoji.trim().to_string();

    if emoji.is_empty() {
        ctx.say("❌ Emoji cannot be empty.").await?;
        return Ok(());
    }

    queries::add_emoji(&ctx.data().db, &guild_id, &emoji).await?;

    ctx.say(format!("✅ Added **{}** to the reaction emojis.", emoji))
        .await?;

    Ok(())
}

/// Remove an emoji from the reaction list.
#[poise::command(slash_command, guild_only, rename = "remove-emoji")]
pub async fn remove_emoji(
    ctx: Context<'_>,
    #[description = "Emoji to remove"] emoji: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let emoji = emoji.trim().to_string();

    queries::remove_emoji(&ctx.data().db, &guild_id, &emoji).await?;

    ctx.say(format!("✅ Removed **{}** from reaction emojis.", emoji))
        .await?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /config interval
// ────────────────────────────────────────────────────────────────────────────

/// Set how often memes are auto-posted (in seconds). Lower = faster posting. Minimum 60s.
#[poise::command(slash_command, guild_only)]
pub async fn interval(
    ctx: Context<'_>,
    #[description = "Interval in seconds between meme posts (min 60, default 60)"] seconds: i64,
) -> Result<(), Error> {
    if seconds < 60 {
        ctx.say("❌ Interval must be at least **60 seconds** to avoid rate-limiting.").await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_posting_interval(&ctx.data().db, &guild_id, seconds).await?;

    let minutes = seconds as f64 / 60.0;
    ctx.say(format!(
        "✅ Meme auto-post interval set to **{} seconds** ({:.1} minutes).\n\
        ⚡ The bot will now post memes every {}s. Use `/admin force-refresh` to post immediately!",
        seconds, minutes, seconds
    ))
    .await?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// /config show
// ────────────────────────────────────────────────────────────────────────────

/// Show the current bot configuration for this server.
/// For a full dashboard, use /admin status.
#[poise::command(slash_command, guild_only)]
pub async fn show(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    let cfg      = queries::get_or_create_guild(db, &guild_id).await?;
    let channels = queries::get_reaction_channels(db, &guild_id).await?;
    let users    = queries::get_reaction_users(db, &guild_id).await?;
    let emojis   = queries::get_emojis(db, &guild_id).await?;

    let meme_ch = cfg
        .meme_channel_id
        .as_ref()
        .and_then(|id| id.parse::<u64>().ok())
        .map(|id| serenity::ChannelId::new(id).mention().to_string())
        .unwrap_or_else(|| "*not set*".to_string());

    let reaction_chs = if channels.is_empty() {
        "*none*".to_string()
    } else {
        channels
            .iter()
            .filter_map(|id| id.parse::<u64>().ok())
            .map(|id| serenity::ChannelId::new(id).mention().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let reaction_users = if users.is_empty() {
        "*none*".to_string()
    } else {
        users
            .iter()
            .filter_map(|id| id.parse::<u64>().ok())
            .map(|id| serenity::UserId::new(id).mention().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let emoji_list = if emojis.is_empty() {
        "👍 *(default)*".to_string()
    } else {
        emojis.join(", ")
    };

    let reply = format!(
        "## ⚙️ Bot Configuration\n\
        **Meme Channel:** {meme_ch}\n\
        **Posting Interval:** {} seconds\n\
        **Reaction Channels:** {reaction_chs}\n\
        **Reaction Users:** {reaction_users}\n\
        **Reaction Emojis:** {emoji_list}\n\n\
        💡 Use `/admin status` for the full multi-channel dashboard.",
        cfg.posting_interval_secs,
    );

    ctx.say(reply).await?;
    Ok(())
}

