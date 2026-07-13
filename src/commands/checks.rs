use crate::data::{Context, Error};
use tracing::{info, warn};

pub async fn is_admin_check(ctx: Context<'_>) -> Result<bool, Error> {
    let author_id = ctx.author().id;
    let guild_id = ctx.guild_id();
    
    info!("Running is_admin_check for user {} in guild {:?}", author_id, guild_id);

    // 1. Owner check (safely get owner_id before any await point to keep future Send)
    let cached_owner_id = ctx.guild().map(|g| g.owner_id);
    info!("Cached guild owner: {:?}", cached_owner_id);

    let is_owner = if let Some(owner_id) = cached_owner_id {
        owner_id == author_id
    } else if let Some(gid) = guild_id {
        // HTTP fallback if cache is empty
        match gid.to_partial_guild(ctx.http()).await {
            Ok(partial) => {
                info!("Fetched partial guild via HTTP. Owner: {}", partial.owner_id);
                partial.owner_id == author_id
            }
            Err(e) => {
                warn!("HTTP fallback: failed to fetch guild details for check: {:?}", e);
                false
            }
        }
    } else {
        false
    };

    info!("is_owner result: {}", is_owner);
    if is_owner {
        return Ok(true);
    }

    // 2. Manage Guild or Admin check
    let has_manage = match ctx {
        poise::Context::Application(app_ctx) => {
            if let Some(member) = &app_ctx.interaction.member {
                if let Some(perms) = member.permissions {
                    let has_perm = perms.manage_guild() || perms.administrator();
                    info!("Slash cmd member permissions: {:?} (manage_guild or admin = {})", perms, has_perm);
                    has_perm
                } else {
                    info!("Slash cmd member permissions field is None");
                    false
                }
            } else {
                info!("Slash cmd interaction member is None");
                false
            }
        }
        poise::Context::Prefix(_) => {
            if let Some(member) = ctx.author_member().await {
                if let Some(perms) = member.permissions {
                    perms.manage_guild() || perms.administrator()
                } else {
                    false
                }
            } else {
                false
            }
        }
    };

    info!("has_manage result: {}", has_manage);
    Ok(has_manage)
}

// ─────────────────────────────────────────────────────────────────────────────
// Global blocklist check — applied to every command via pre_command hook
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `Ok(false)` (silently blocks) if the invoking user is on the
/// guild's blocklist. Admins and the server owner are never blocked.
pub async fn is_not_blocked_check(ctx: crate::data::Context<'_>) -> Result<bool, crate::data::Error> {
    // Only applies inside guilds
    let guild_id = match ctx.guild_id() {
        Some(id) => id.to_string(),
        None => return Ok(true), // allow DM interactions (there are none, but be safe)
    };

    let user_id = ctx.author().id.to_string();
    let db = &ctx.data().db;

    match crate::db::queries::is_user_blocked(db, &guild_id, &user_id).await {
        Ok(true) => {
            info!("Blocked user {} tried to use /{} in {}", user_id, ctx.command().name, guild_id);
            // Reply ephemerally so only they see it, then return false
            let _ = ctx.send(
                poise::CreateReply::default()
                    .content("🚫 You have been blocked from using bot commands in this server.")
                    .ephemeral(true)
            ).await;
            Ok(false)
        }
        Ok(false) => Ok(true),
        Err(e) => {
            warn!("DB error in is_not_blocked_check: {:?}", e);
            Ok(true) // fail open — don't block on DB errors
        }
    }
}
