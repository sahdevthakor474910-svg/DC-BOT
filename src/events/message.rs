use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{debug, error, info};

use crate::data::Data;
use crate::db::queries;
use crate::dmc::{calculator::BossStats, gemini};

/// Called for every new message received by the bot.
///
/// 1. If the message has image attachments → try DMC: Peak of Combat analysis.
/// 2. Applies auto-reactions based on guild configuration.
pub async fn handle(
    ctx: &serenity::Context,
    message: &serenity::Message,
    data: &Data,
) -> Result<()> {
    // Ignore messages from bots (including ourselves)
    if message.author.bot {
        return Ok(());
    }

    // ── DMC screenshot analysis ───────────────────────────────────────────────
    // Check if the message has any image attachments and we have a Gemini key.
    if !data.config.gemini_api_key.is_empty() {
        let image_attachments: Vec<&serenity::Attachment> = message
            .attachments
            .iter()
            .filter(|a| {
                matches!(
                    a.content_type.as_deref(),
                    Some("image/png")
                        | Some("image/jpeg")
                        | Some("image/jpg")
                        | Some("image/gif")
                        | Some("image/webp")
                )
                // Fallback: check file extension for attachments without content_type
                || {
                    let name = a.filename.to_lowercase();
                    name.ends_with(".png")
                        || name.ends_with(".jpg")
                        || name.ends_with(".jpeg")
                        || name.ends_with(".gif")
                        || name.ends_with(".webp")
                }
            })
            .collect();

        if !image_attachments.is_empty() {
            for attachment in image_attachments {
                info!(
                    "🎮 DMC screenshot detected: '{}' from {}",
                    attachment.filename,
                    message.author.name
                );

                match gemini::analyze_screenshot(
                    &data.http_client,
                    &data.config.gemini_api_key,
                    &attachment.url,
                )
                .await
                {
                    Ok(boss_result) => {
                        let stats = BossStats::compute(&boss_result);
                        let reply = stats.discord_message();

                        if let Err(e) = message.reply(&ctx.http, &reply).await {
                            error!("Failed to send DMC analysis reply: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("DMC analysis failed: {:#}", e);
                        // Reply visibly so you can see what went wrong while testing
                        let _ = message
                            .reply(&ctx.http, format!("❌ DMC analysis error: `{}`", e))
                            .await;
                    }
                }
            }
        }
    }

    // Only the remainder operates inside guilds for auto-reacting
    let guild_id = match message.guild_id {
        Some(id) => id.to_string(),
        None => return Ok(()),
    };

    let channel_id = message.channel_id.to_string();
    let user_id    = message.author.id.to_string();

    // Check global auto-react toggle for this guild
    let cfg = queries::get_or_create_guild(&data.db, &guild_id).await?;
    if !cfg.auto_react_enabled {
        return Ok(());
    }

    // Determine if this message qualifies for auto-reaction
    let in_reaction_channel = queries::is_reaction_channel(&data.db, &guild_id, &channel_id).await?;
    let is_reaction_user    = queries::is_reaction_user(&data.db, &guild_id, &user_id).await?;

    if !in_reaction_channel && !is_reaction_user {
        return Ok(());
    }

    debug!(
        "Auto-reacting: msg={} guild={} ch_match={} user_match={}",
        message.id, guild_id, in_reaction_channel, is_reaction_user
    );

    // Build final emoji list — combine per-channel and per-user overrides,
    // falling back to the guild-wide default, then to 👍.
    let mut emoji_set: Vec<String> = Vec::new();

    if in_reaction_channel {
        let ch_emojis = queries::get_channel_emojis(&data.db, &guild_id, &channel_id).await?;
        emoji_set.extend(ch_emojis);
    }

    if is_reaction_user {
        let user_emojis = queries::get_user_emojis(&data.db, &guild_id, &user_id).await?;
        for e in user_emojis {
            if !emoji_set.contains(&e) {
                emoji_set.push(e);
            }
        }
    }

    // Fall back to guild-wide default
    if emoji_set.is_empty() {
        emoji_set = queries::get_emojis(&data.db, &guild_id).await?;
    }

    // Ultimate fallback
    if emoji_set.is_empty() {
        emoji_set = vec!["👍".to_string()];
    }

    for emoji_str in emoji_set {
        let reaction = parse_reaction(&emoji_str);
        if let Err(e) = message.react(&ctx.http, reaction).await {
            let err_str = e.to_string();

            // Unknown/deleted custom emoji — fall back to 👍 and continue
            if err_str.contains("Unknown Emoji") || err_str.contains("10014") {
                tracing::warn!(
                    "Custom emoji '{}' is unknown/deleted, falling back to 👍 (msg {})",
                    emoji_str, message.id
                );
                let _ = message.react(&ctx.http, serenity::ReactionType::Unicode("👍".to_string())).await;
                continue;
            }

            // User blocked bot or bot lacks permissions for THIS reaction — skip, try next
            if err_str.contains("50007")
                || err_str.contains("blocked")
                || err_str.contains("Missing Permissions")
                || err_str.contains("Missing Access")
                || err_str.contains("50013")
            {
                tracing::warn!(
                    "Skipping reaction '{}' on msg {} (blocked/no perms: {})",
                    emoji_str, message.id, err_str
                );
                continue;
            }

            // Any other error — log and still continue with remaining emojis
            error!(
                "Failed to react with '{}' to message {}: {}",
                emoji_str, message.id, e
            );
        }
    }

    Ok(())
}

/// Parse an emoji string into a `ReactionType`.
/// Supports:
///   - Unicode emoji:    "🔥"
///   - Custom emoji:     "<:name:12345>" or "<a:name:12345>" (animated)
fn parse_reaction(s: &str) -> serenity::ReactionType {
    let s = s.trim();
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        let animated = inner.starts_with("a:");
        let inner = if animated {
            &inner[2..]
        } else {
            inner.trim_start_matches(':')
        };
        let mut parts = inner.splitn(2, ':');
        if let (Some(name), Some(id_str)) = (parts.next(), parts.next()) {
            if let Ok(id) = id_str.parse::<u64>() {
                return serenity::ReactionType::Custom {
                    animated,
                    id: serenity::EmojiId::new(id),
                    name: Some(name.to_string()),
                };
            }
        }
    }
    serenity::ReactionType::Unicode(s.to_string())
}
