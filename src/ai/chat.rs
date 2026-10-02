use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{debug, error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::actions;
use super::client::AiClient;

/// Check if a message is directed to the bot (either via mention or reply).
/// If so, handle it via AI and return `Ok(true)`.
/// If not, return `Ok(false)` so other message handlers (like auto-react) can proceed.
pub async fn handle_ai_message(
    ctx: &serenity::Context,
    message: &serenity::Message,
    data: &Data,
) -> Result<bool> {
    // 1. Get current bot user ID
    let bot_user_id = ctx.cache.current_user().id;

    // 2. Check if bot was mentioned
    let is_mentioned = message.mentions.iter().any(|u| u.id == bot_user_id);

    // 3. Check if message is a reply to the bot
    let is_reply_to_bot = message
        .referenced_message
        .as_ref()
        .map_or(false, |ref_msg| ref_msg.author.id == bot_user_id);

    if !is_mentioned && !is_reply_to_bot {
        return Ok(false);
    }

    info!(
        "🤖 AI trigger from {} ({}) in channel {}: mention={}, reply={}",
        message.author.name, message.author.id, message.channel_id, is_mentioned, is_reply_to_bot
    );

    // Check if user is blocked in this guild
    if let Some(guild_id) = message.guild_id {
        let guild_id_str = guild_id.to_string();
        let user_id_str = message.author.id.to_string();
        if let Ok(true) = queries::is_user_blocked(&data.db, &guild_id_str, &user_id_str).await {
            debug!("Ignoring AI message from blocked user {}", user_id_str);
            return Ok(true);
        }
    }

    // Clean user prompt by removing the bot mention tokens
    let mut clean_prompt = message.content.clone();
    let mention_plain = format!("<@{}>", bot_user_id);
    let mention_nick = format!("<@!{}>", bot_user_id);
    clean_prompt = clean_prompt.replace(&mention_plain, "").replace(&mention_nick, "");
    let clean_prompt = clean_prompt.trim().to_string();

    // If empty prompt (user just pinged the bot with no text)
    if clean_prompt.is_empty() {
        let greeting = format!(
            "👋 Hello **{}**! How can I help you today? You can ask me any question or chat with me!",
            message.author.name
        );
        let _ = message.reply(&ctx.http, greeting).await;
        return Ok(true);
    }

    // Check if Gemini API key is configured
    if data.config.gemini_api_key.trim().is_empty() {
        let no_key_msg = "⚠️ AI Chat is currently disabled because `GEMINI_API_KEY` is not set in the bot's environment variables. Please add a free Gemini API key to your Render dashboard to enable this feature!";
        let _ = message.reply(&ctx.http, no_key_msg).await;
        return Ok(true);
    }

    // Determine if the user is the server owner
    let (is_owner, guild_name) = if let Some(guild_id) = message.guild_id {
        match guild_id.to_partial_guild(&ctx.http).await {
            Ok(guild) => (guild.owner_id == message.author.id, guild.name),
            Err(e) => {
                warn!("Could not fetch guild info for {}: {}", guild_id, e);
                (false, "Server".to_string())
            }
        }
    } else {
        (false, "Direct Message".to_string())
    };

    // Check if user is a designated menace target
    let is_menace_target = if let Some(guild_id) = message.guild_id {
        queries::is_menace_user(&data.db, &guild_id.to_string(), &message.author.id.to_string())
            .await
            .unwrap_or(false)
    } else {
        false
    };

    // Extract reference text if user is replying to a previous message
    let reply_context = message
        .referenced_message
        .as_ref()
        .map(|rm| rm.content.as_str());

    // Build context header
    let context_header = format!(
        "SERVER: {}\nUSER: {} (ID: {})\n[SERVER OWNER: {}]\n[MENACE ROAST TARGET: {}]",
        guild_name,
        message.author.name,
        message.author.id,
        if is_owner { "YES" } else { "NO" },
        if is_menace_target { "YES - THIS USER IS ON YOUR HIT LIST! ROAST THEM UNMERCIFULLY IN ENGLISH/HINDI" } else { "NO" }
    );

    // Broadcast typing indicator while AI generates
    let _ = ctx.http.broadcast_typing(message.channel_id).await;

    let ai_client = AiClient::new(data.config.gemini_api_key.clone());

    match ai_client.chat(&context_header, &clean_prompt, reply_context).await {
        Ok((mut response_text, action_opt)) => {
            // If an action was extracted
            if let Some(action) = action_opt {
                if is_owner {
                    if let Some(guild_id) = message.guild_id {
                        match actions::execute_action(ctx, &data.db, guild_id, message.channel_id, action).await {
                            Ok(act_msg) => {
                                response_text.push_str(&format!("\n\n{}", act_msg));
                            }
                            Err(e) => {
                                response_text.push_str(&format!("\n\n⚠️ **Action Failed:** {}", e));
                            }
                        }
                    } else {
                        response_text.push_str("\n\n⚠️ Server management actions can only be used inside a Discord server.");
                    }
                } else {
                    response_text.push_str("\n\n🔒 *Server management actions are restricted to the server owner.*");
                }
            }

            // Send reply to Discord, splitting if over 2000 chars
            send_chunked_reply(ctx, message, &response_text).await?;
        }
        Err(e) => {
            error!("AI generation failed: {:#}", e);
            let err_reply = format!(
                "⚠️ **AI Error:** {}\n*(Please check that `GEMINI_API_KEY` on Render is valid and active at [aistudio.google.com](https://aistudio.google.com/))*",
                e
            );
            let _ = message.reply(&ctx.http, err_reply).await;
        }
    }

    Ok(true)
}

/// Helper to reply to a message and chunk if exceeding Discord's 2000 character limit
async fn send_chunked_reply(
    ctx: &serenity::Context,
    message: &serenity::Message,
    text: &str,
) -> Result<()> {
    if text.len() <= 2000 {
        message.reply(&ctx.http, text).await?;
        return Ok(());
    }

    let mut remaining = text;
    let mut is_first = true;

    while !remaining.is_empty() {
        let max_len = remaining.floor_char_boundary(1950.min(remaining.len()));
        let chunk_size = if remaining.len() > max_len {
            let substr = &remaining[..max_len];
            substr.rfind('\n').unwrap_or_else(|| substr.rfind(' ').unwrap_or(max_len))
        } else {
            remaining.len()
        };

        let chunk = remaining[..chunk_size].trim();
        if !chunk.is_empty() {
            if is_first {
                message.reply(&ctx.http, chunk).await?;
                is_first = false;
            } else {
                message.channel_id.say(&ctx.http, chunk).await?;
            }
        }

        let next_start = remaining.ceil_char_boundary(chunk_size.min(remaining.len()));
        remaining = remaining[next_start..].trim_start();
    }

    Ok(())
}
