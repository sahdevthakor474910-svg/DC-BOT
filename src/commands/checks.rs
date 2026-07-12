use crate::data::{Context, Error};

pub async fn is_admin_check(ctx: Context<'_>) -> Result<bool, Error> {
    // 1. Owner check (safely get owner_id before any await point to keep future Send)
    let cached_owner_id = ctx.guild().map(|g| g.owner_id);

    let is_owner = if let Some(owner_id) = cached_owner_id {
        owner_id == ctx.author().id
    } else if let Some(guild_id) = ctx.guild_id() {
        // HTTP fallback if cache is empty
        if let Ok(partial) = guild_id.to_partial_guild(ctx.http()).await {
            partial.owner_id == ctx.author().id
        } else {
            false
        }
    } else {
        false
    };

    if is_owner {
        return Ok(true);
    }

    // 2. Manage Guild or Admin check
    let has_manage = match ctx {
        poise::Context::Application(app_ctx) => {
            if let Some(member) = &app_ctx.interaction.member {
                member.permissions.map(|p| p.manage_guild() || p.administrator()).unwrap_or(false)
            } else {
                false
            }
        }
        poise::Context::Prefix(_) => {
            if let Some(member) = ctx.author_member().await {
                member.permissions.map(|p| p.manage_guild() || p.administrator()).unwrap_or(false)
            } else {
                false
            }
        }
    };

    Ok(has_manage)
}
