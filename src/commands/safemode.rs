use crate::data::{Context, Error};
use crate::db::queries;

/// 🛡️ Toggle Safe Mode ON or OFF (suppresses gali/slangs & switches bot to clean PG mode).
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check",
    rename = "safemode"
)]
pub async fn safemode(
    ctx: Context<'_>,
    #[description = "Set safe mode enabled (true/false). Leave empty to toggle."]
    enabled: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let cfg = queries::get_or_create_guild(&ctx.data().db, &guild_id).await?;
    let new_state = enabled.unwrap_or(!cfg.safe_mode_enabled);
    queries::set_safe_mode_enabled(&ctx.data().db, &guild_id, new_state).await?;

    if new_state {
        ctx.say("🛡️ **Safe Mode is now ENABLED!**\n• Gali / abusive slang auto-responses are **disabled**.\n• AI responses switched to clean, witty, family-friendly PG mode.\n• No slangs, swearing, or adult insults.").await?;
    } else {
        ctx.say("⚡ **Safe Mode is now DISABLED!**\n• Full savage mode active — raw street slangs, unfiltered roasts, and gali comebacks enabled!").await?;
    }
    Ok(())
}
