use crate::data::{Context, Error};

/// Poise check: passes if the invoker is the guild owner
/// OR has Manage Server / Administrator permission.
/// Safe for server owners regardless of role assignments.
pub async fn is_admin_check(ctx: Context<'_>) -> Result<bool, Error> {
    let is_owner = ctx
        .guild()
        .map(|g| g.owner_id == ctx.author().id)
        .unwrap_or(false);

    if is_owner {
        return Ok(true);
    }

    let has_manage = ctx
        .author_member()
        .await
        .and_then(|m| m.permissions)
        .map(|p| p.manage_guild() || p.administrator())
        .unwrap_or(false);

    Ok(has_manage)
}
