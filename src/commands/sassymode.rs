use crate::data::{Context, Error};
use crate::db::queries;

/// 💅 Toggle Sassy Flirt Mode ON or OFF (100000000% cheesy, flirty gay sassy persona).
#[poise::command(
    slash_command,
    guild_only,
    check = "crate::commands::checks::is_admin_check",
    description_localized("en-US", "Toggle Sassy Mode ON/OFF (100000000% cheesy, flirty gay sassy persona)")
)]
pub async fn sassymode(
    ctx: Context<'_>,
    #[description = "Set sassy mode enabled (true/false). Leave empty to toggle."]
    enabled: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let cfg = queries::get_or_create_guild(&ctx.data().db, &guild_id).await?;
    let new_state = enabled.unwrap_or(!cfg.sassy_mode_enabled);
    queries::set_sassy_mode_enabled(&ctx.data().db, &guild_id, new_state).await?;

    if new_state {
        ctx.say("💅 **Sassy Mode is now ENABLED!**\n• Honey, brace yourself — the bot is now **100000000% cheesy, flirty, and sassy**!\n• Unapologetic gay bestie / queen energy with outrageous cheesy pickup lines on every query! 😉💋✨").await?;
    } else {
        ctx.say("🖤 **Sassy Mode is now DISABLED!**\n• Bot has returned to normal casual mode.").await?;
    }
    Ok(())
}
