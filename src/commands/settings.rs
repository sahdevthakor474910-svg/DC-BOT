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
    check = "crate::commands::checks::is_admin_check",
    subcommands(
        "interval",
        "auto_react",
        "add_react_channel",
        "remove_react_channel",
        "add_react_user",
        "remove_react_user",
        "add_emoji",
        "remove_emoji",
        "clear_cache",
        "block_user",
        "unblock_user",
        "blocked_list",
        "export_setup",
        "import_setup"
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

// ─────────────────────────────────────────────────────────────────────────────

/// 🧹 Clear history cache to re-post the same hot entries again for testing.
#[poise::command(slash_command, guild_only, rename = "clear-cache")]
pub async fn clear_cache(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    queries::clear_guild_seen_cache(&ctx.data().db, &guild_id).await?;
    ctx.say("✅ **Cache cleared successfully!** The bot's post history memory has been wiped. Run `/post` now to instantly post all top/hot items!").await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 🚫 Block a user from using any bot commands in this server.
#[poise::command(slash_command, guild_only, rename = "block-user")]
pub async fn block_user(
    ctx: Context<'_>,
    #[description = "User to block from bot commands"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let user_id  = user.id.to_string();

    // Prevent blocking yourself or the bot
    if user.id == ctx.author().id {
        ctx.say("❌ You can't block yourself.").await?;
        return Ok(());
    }
    if user.bot {
        ctx.say("❌ You can't block a bot.").await?;
        return Ok(());
    }

    queries::block_user(&ctx.data().db, &guild_id, &user_id).await?;
    ctx.say(format!("🚫 {} has been **blocked** from using bot commands in this server.", user.mention())).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 🔓 Unblock a previously blocked user, restoring their access to bot commands.
#[poise::command(slash_command, guild_only, rename = "unblock-user")]
pub async fn unblock_user(
    ctx: Context<'_>,
    #[description = "User to unblock"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let user_id  = user.id.to_string();

    queries::unblock_user(&ctx.data().db, &guild_id, &user_id).await?;
    ctx.say(format!("✅ {} has been **unblocked** and can use bot commands again.", user.mention())).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 📋 Show all users currently blocked from bot commands in this server.
#[poise::command(slash_command, guild_only, rename = "blocked-list")]
pub async fn blocked_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let blocked  = queries::get_blocked_users(&ctx.data().db, &guild_id).await?;

    if blocked.is_empty() {
        ctx.say("✅ No users are currently blocked from bot commands.").await?;
        return Ok(());
    }

    let list = blocked
        .iter()
        .filter_map(|id| id.parse::<u64>().ok())
        .map(|id| format!("• {}", serenity::UserId::new(id).mention()))
        .collect::<Vec<_>>()
        .join("\n");

    let embed = serenity::CreateEmbed::new()
        .title("🚫 Blocked Users")
        .description(format!("The following users are blocked from using bot commands:\n\n{}", list))
        .color(0xED4245) // Discord red
        .footer(serenity::CreateEmbedFooter::new(
            "Use /settings unblock-user @user to restore access"
        ));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// 📤 Export the entire bot setup configurations for this server to a JSON file.
#[poise::command(slash_command, guild_only, rename = "export-setup")]
pub async fn export_setup(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    ctx.defer().await?;

    let backup = queries::export_guild_setup(db, &guild_id).await?;
    let json_bytes = serde_json::to_vec_pretty(&backup)?;

    let attachment = serenity::CreateAttachment::bytes(json_bytes, "setup_backup.json");
    ctx.send(
        poise::CreateReply::default()
            .content("📤 **Here is your server setup configurations file!**\nKeep this file stored safely. If you redeploy or the bot's data resets, you can import this file using `/settings import-setup` to restore all settings.")
            .attachment(attachment)
    ).await?;

    Ok(())
}

/// 📥 Import/Restore the bot setup configurations from a previously exported JSON file.
#[poise::command(slash_command, guild_only, rename = "import-setup")]
pub async fn import_setup(
    ctx: Context<'_>,
    #[description = "The exported setup_backup.json file"] attachment: serenity::Attachment,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    ctx.defer().await?;

    let file_bytes = match attachment.download().await {
        Ok(bytes) => bytes,
        Err(e) => {
            ctx.say(format!("❌ Failed to download attachment: {}", e)).await?;
            return Ok(());
        }
    };

    let backup: queries::GuildSetupBackup = match serde_json::from_slice(&file_bytes) {
        Ok(b) => b,
        Err(e) => {
            ctx.say(format!("❌ Invalid backup file format: {}. Make sure you are uploading the correct `setup_backup.json` file.", e)).await?;
            return Ok(());
        }
    };

    if let Err(e) = queries::import_guild_setup(db, &guild_id, backup).await {
        ctx.say(format!("❌ Failed to restore configuration: {}", e)).await?;
        return Ok(());
    }

    ctx.say("✅ **Setup restored successfully!** All channel mappings, auto-react targets, emojis and settings are back in place.").await?;
    Ok(())
}

