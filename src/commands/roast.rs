use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;
use crate::data::{Context, Error};

/// 🔥 Unleash a savage Desi / Hindi roast on someone or yourself!
#[poise::command(slash_command, guild_only, description_localized("en-US", "Unleash a savage roast"))]
pub async fn roast(
    ctx: Context<'_>,
    #[description = "User to roast (leave empty to roast yourself)"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().to_string();
    let is_sassy = crate::db::queries::get_or_create_guild(&ctx.data().db, &guild_id)
        .await
        .map(|c| c.sassy_mode_enabled)
        .unwrap_or(false);

    let roast = if is_sassy {
        crate::gali::get_random_sassy_comeback()
    } else {
        crate::gali::get_random_gali_comeback()
    };

    let target = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{} {}", target.mention(), roast);

    ctx.say(response).await?;
    Ok(())
}
