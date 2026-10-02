use anyhow::{anyhow, Result};
use poise::serenity_prelude as serenity;
use tracing::{error, info};

use super::models::ServerAction;

/// Executes an authorized server management action on behalf of the server owner.
pub async fn execute_action(
    ctx: &serenity::Context,
    db: &sqlx::SqlitePool,
    guild_id: serenity::GuildId,
    current_channel_id: serenity::ChannelId,
    action: ServerAction,
) -> Result<String> {
    match action {
        ServerAction::CreateChannel { name, kind, topic } => {
            let channel_type = match kind.as_deref().unwrap_or("text") {
                "voice" => serenity::ChannelType::Voice,
                "category" => serenity::ChannelType::Category,
                _ => serenity::ChannelType::Text,
            };

            let mut builder = serenity::CreateChannel::new(&name).kind(channel_type);
            if let Some(t) = topic {
                builder = builder.topic(t);
            }

            match guild_id.create_channel(&ctx.http, builder).await {
                Ok(ch) => {
                    info!("🛠️ Created channel {} ({}) in guild {}", ch.name, ch.id, guild_id);
                    Ok(format!("✅ Successfully created channel **#{}** ({:?})!", ch.name, channel_type))
                }
                Err(e) => {
                    error!("Failed to create channel '{}': {}", name, e);
                    Err(anyhow!("Failed to create channel '{}': {}", name, e))
                }
            }
        }

        ServerAction::DeleteChannel { name } => {
            let channels = guild_id.channels(&ctx.http).await?;
            let target_name = name.trim().trim_start_matches('#').to_lowercase();

            let target = channels.values().find(|c| {
                c.name.to_lowercase() == target_name || c.id.to_string() == target_name
            });

            match target {
                Some(ch) => {
                    let deleted_name = ch.name.clone();
                    ch.id.delete(&ctx.http).await?;
                    info!("🛠️ Deleted channel #{} in guild {}", deleted_name, guild_id);
                    Ok(format!("🗑️ Successfully deleted channel **#{}**!", deleted_name))
                }
                None => Err(anyhow!("Could not find channel '{}' in this server.", name)),
            }
        }

        ServerAction::RenameChannel { old_name, new_name } => {
            let channels = guild_id.channels(&ctx.http).await?;
            let target_old = old_name.trim().trim_start_matches('#').to_lowercase();

            let target = channels.values().find(|c| {
                c.name.to_lowercase() == target_old || c.id.to_string() == target_old
            });

            match target {
                Some(ch) => {
                    let clean_new = new_name.trim().trim_start_matches('#').to_string();
                    let edit = serenity::EditChannel::new().name(&clean_new);
                    ch.id.edit(&ctx.http, edit).await?;
                    info!("🛠️ Renamed channel #{} to #{} in guild {}", old_name, clean_new, guild_id);
                    Ok(format!("✏️ Renamed channel from **#{}** to **#{}**!", old_name, clean_new))
                }
                None => Err(anyhow!("Could not find channel '{}' in this server.", old_name)),
            }
        }

        ServerAction::SetSlowmode { channel, seconds } => {
            let target_channel_id = if let Some(ch_name) = channel {
                let channels = guild_id.channels(&ctx.http).await?;
                let clean_name = ch_name.trim().trim_start_matches('#').to_lowercase();
                channels
                    .values()
                    .find(|c| c.name.to_lowercase() == clean_name || c.id.to_string() == clean_name)
                    .map(|c| c.id)
                    .unwrap_or(current_channel_id)
            } else {
                current_channel_id
            };

            let edit = serenity::EditChannel::new().rate_limit_per_user(seconds);
            target_channel_id.edit(&ctx.http, edit).await?;
            info!("🛠️ Set slowmode to {}s for channel {} in guild {}", seconds, target_channel_id, guild_id);
            Ok(format!("⏱️ Set slowmode in <#{}> to **{} seconds**!", target_channel_id, seconds))
        }

        ServerAction::KickUser { user, reason } => {
            let user_id = parse_user_id(&user)?;
            let reason_str = reason.as_deref().unwrap_or("Kicked by server owner via AI command");
            guild_id.kick_with_reason(&ctx.http, user_id, reason_str).await?;
            info!("🛠️ Kicked user {} from guild {}", user_id, guild_id);
            Ok(format!("👢 Successfully kicked <@{}> from the server! Reason: {}", user_id, reason_str))
        }

        ServerAction::BanUser { user, reason } => {
            let user_id = parse_user_id(&user)?;
            let reason_str = reason.as_deref().unwrap_or("Banned by server owner via AI command");
            guild_id.ban_with_reason(&ctx.http, user_id, 0, reason_str).await?;
            info!("🛠️ Banned user {} from guild {}", user_id, guild_id);
            Ok(format!("🔨 Successfully banned <@{}> from the server! Reason: {}", user_id, reason_str))
        }

        ServerAction::AddMenaceUser { user } => {
            let user_id = parse_user_id(&user)?;
            let guild_id_str = guild_id.to_string();
            let user_id_str = user_id.to_string();
            crate::db::queries::add_menace_user(db, &guild_id_str, &user_id_str).await?;
            info!("😈 Added user {} to menace targets in guild {}", user_id, guild_id);
            Ok(format!("😈 **<@{}>** has been added to the **Menace Hit List**! I will now ruthlessly flame and roast them on sight in English/Hindi.", user_id))
        }

        ServerAction::RemoveMenaceUser { user } => {
            let user_id = parse_user_id(&user)?;
            let guild_id_str = guild_id.to_string();
            let user_id_str = user_id.to_string();
            crate::db::queries::remove_menace_user(db, &guild_id_str, &user_id_str).await?;
            info!("😇 Removed user {} from menace targets in guild {}", user_id, guild_id);
            Ok(format!("😇 **<@{}>** has been spared and removed from the menace list.", user_id))
        }
    }
}

/// Helper to parse a user ID from a mention (<@123456> or <@!123456>) or plain ID string
fn parse_user_id(s: &str) -> Result<serenity::UserId> {
    let clean = s
        .trim()
        .trim_start_matches("<@!")
        .trim_start_matches("<@")
        .trim_end_matches('>');
    match clean.parse::<u64>() {
        Ok(id) => Ok(serenity::UserId::new(id)),
        Err(_) => Err(anyhow!("Invalid user format or ID '{}'. Please mention the user or supply their ID.", s)),
    }
}
