use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{debug, error};

use crate::data::Data;
use crate::db::queries;

/// Called for every new message received by the bot.
/// Applies auto-reactions based on guild configuration.
pub async fn handle(
    ctx: &serenity::Context,
    message: &serenity::Message,
    data: &Data,
) -> Result<()> {
    // Ignore messages from bots (including ourselves)
    if message.author.bot {
        return Ok(());
    }

    // Only operate inside guilds
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
