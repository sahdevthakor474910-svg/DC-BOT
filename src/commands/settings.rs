use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;

use crate::data::{Context, Error};
use crate::db::queries;

// ─────────────────────────────────────────────────────────────────────────────
// /settings — all bot settings in one place (reactions, emojis, interval)
// ─────────────────────────────────────────────────────────────────────────────

/// ⚙️ Bot settings — configure reactions, emojis, and post speed.
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "MANAGE_GUILD",
    subcommands(
        "interval",
        "auto_react",
        "add_react_channel",
        "remove_react_channel",
        "add_react_user",
        "remove_react_user",
        "add_emoji",
        "remove_emoji"
    )
)]
pub async fn settings(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// ⏱️ Set how fast memes auto-post. Lower = more frequent. Minimum 60 seconds.
#[poise::command(slash_command, guild_only, rename = "interval")]
pub async fn interval(
    ctx: Context<'_>,
    #[description = "Seconds between meme posts (e.g. 60 = every minute, 300 = every 5 min)"]
    seconds: i64,
) -> Result<(), Error> {
    if seconds < 60 {
        ctx.say("❌ Minimum is **60 seconds** to avoid Discord rate limits.").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::set_posting_interval(&ctx.data().db, &guild_id, seconds).await?;
    ctx.say(format!(
        "✅ Memes will now auto-post every **{}s**.\n⚡ Use `/post` to post right now!",
        seconds
    )).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 😄 Toggle automatic emoji reactions ON or OFF.
#[poise::command(slash_command, guild_only, rename = "auto-react")]
pub async fn auto_react(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let cfg = queries::get_or_create_guild(&ctx.data().db, &guild_id).await?;
    let new_state = !cfg.auto_react_enabled;
    queries::set_auto_react_enabled(&ctx.data().db, &guild_id, new_state).await?;
    let label = if new_state { "🟢 **enabled**" } else { "🔴 **disabled**" };
    ctx.say(format!("Auto-react is now {}", label)).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 📢 Make the bot react to every message in a channel.
#[poise::command(slash_command, guild_only, rename = "add-react-channel")]
pub async fn add_react_channel(
    ctx: Context<'_>,
    #[description = "Channel to auto-react in"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::add_reaction_channel(&ctx.data().db, &guild_id, &channel.id.to_string()).await?;
    ctx.say(format!("✅ Bot will now react to every message in {}.", channel.id.mention())).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 🔇 Stop auto-reacting in a channel.
#[poise::command(slash_command, guild_only, rename = "remove-react-channel")]
pub async fn remove_react_channel(
    ctx: Context<'_>,
    #[description = "Channel to stop reacting in"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::remove_reaction_channel(&ctx.data().db, &guild_id, &channel.id.to_string()).await?;
    ctx.say(format!("✅ Stopped reacting in {}.", channel.id.mention())).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 👤 React to every message from a specific user.
#[poise::command(slash_command, guild_only, rename = "add-react-user")]
pub async fn add_react_user(
    ctx: Context<'_>,
    #[description = "User to always react to"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::add_reaction_user(&ctx.data().db, &guild_id, &user.id.to_string()).await?;
    ctx.say(format!("✅ Will now react to every message from {}.", user.mention())).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 🚫 Stop reacting to a specific user's messages.
#[poise::command(slash_command, guild_only, rename = "remove-react-user")]
pub async fn remove_react_user(
    ctx: Context<'_>,
    #[description = "User to remove from auto-react"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::remove_reaction_user(&ctx.data().db, &guild_id, &user.id.to_string()).await?;
    ctx.say(format!("✅ Stopped reacting to {}.", user.mention())).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// ➕ Add an emoji to the bot's reaction pool (e.g. 🔥 or <:custom:id>).
#[poise::command(slash_command, guild_only, rename = "add-emoji")]
pub async fn add_emoji(
    ctx: Context<'_>,
    #[description = "Emoji to add (e.g. 🔥 or <:name:id>)"] emoji: String,
) -> Result<(), Error> {
    let emoji = emoji.trim().to_string();
    if emoji.is_empty() {
        ctx.say("❌ Emoji cannot be empty.").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::add_emoji(&ctx.data().db, &guild_id, &emoji).await?;
    ctx.say(format!("✅ Added {} to the reaction pool.", emoji)).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// ➖ Remove an emoji from the bot's reaction pool.
#[poise::command(slash_command, guild_only, rename = "remove-emoji")]
pub async fn remove_emoji(
    ctx: Context<'_>,
    #[description = "Emoji to remove"] emoji: String,
) -> Result<(), Error> {
    let emoji = emoji.trim().to_string();
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::remove_emoji(&ctx.data().db, &guild_id, &emoji).await?;
    ctx.say(format!("✅ Removed {} from the reaction pool.", emoji)).await?;
    Ok(())
}
