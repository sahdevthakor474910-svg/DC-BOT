use crate::data::{Context, Error};
use crate::db::queries;

/// 💬 Toggle Bot Responses ON or OFF (completely enables or mutes chat replies, AI, and gali).
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check",
    description_localized("en-US", "Toggle Bot Responses ON or OFF in chat (mentions, replies & gali)")
)]
pub async fn response(
    ctx: Context<'_>,
    #[description = "Enable bot responses (true/false). Leave empty to toggle."]
    enabled: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let cfg = queries::get_or_create_guild(&ctx.data().db, &guild_id).await?;
    let new_state = enabled.unwrap_or(!cfg.bot_response_enabled);
    queries::set_bot_response_enabled(&ctx.data().db, &guild_id, new_state).await?;

    if new_state {
        ctx.say("🔊 **Bot Responses are now ENABLED!**\n• The bot is active and will respond to mentions, replies, and questions in chat.\n• To mute the bot completely, use `/response false`.").await?;
    } else {
        ctx.say("🔇 **Bot Responses are now DISABLED!**\n• The bot is now completely silent in chat (no AI replies, no mentions response, no gali comebacks).\n• It will silently continue running background tasks (memes, news, free games, auto-posts).\n• Use `/response true` anytime to re-enable chat responses.").await?;
    }
    Ok(())
}
