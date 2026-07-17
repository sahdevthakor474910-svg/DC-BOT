use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::fetcher;

/// Single tick — exposed for `/post`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    tick(data, http, force).await
}

/// Background loop — runs every 10 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("⚔️  Clash of Clans update task started (checking every 10 min)");

    loop {
        match tick(&data, &http, false).await {
            Ok(n) if n > 0 => info!("⚔️  Posted {} CoC update(s)", n),
            Ok(_)          => {}
            Err(e)         => error!("CoC task error: {:#}", e),
        }

        // Prune seen cache older than 30 days
        if let Err(e) = queries::prune_old_seen_coc(&data.db, 30).await {
            warn!("Could not prune seen_coc: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(10 * 60)).await;
    }
}

/// Embed accent colour per tag.
fn tag_colour(tag: &str) -> u32 {
    match tag {
        "🎁 Free Reward"       => 0xF1C40F, // gold
        "⚔️ Update"            => 0xE74C3C, // CoC red
        "🏆 Clan War League"   => 0x9B59B6, // purple
        "🏅 Event"             => 0x2ECC71, // green
        "📢 Announcement"      => 0x3498DB, // blue
        "📺 Update Video"      => 0xFF0000, // YouTube red
        _                      => 0x5865F2, // Discord blurple
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.coc_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let updates = fetcher::fetch_all_updates(&data.http_client).await;
    if updates.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.coc_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid CoC channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);

        for update in &updates {
            if !force {
                if queries::is_coc_seen(&data.db, &cfg.guild_id, &update.id).await? {
                    continue;
                }
            }

            queries::mark_coc_seen(&data.db, &cfg.guild_id, &update.id).await?;

            // Build embed
            let mut embed = serenity::CreateEmbed::new()
                .title(format!("{} {}", update.tag, update.title))
                .url(&update.url)
                .color(tag_colour(&update.tag))
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "⚔️ Clash of Clans • {}",
                    update.source
                )));

            if let Some(desc) = &update.description {
                embed = embed.description(desc);
            }
            if let Some(img) = &update.image_url {
                embed = embed.thumbnail(img);
            }
            if let Some(ts) = update.published_at {
                if let Ok(timestamp) = serenity::Timestamp::from_unix_timestamp(ts.timestamp()) {
                    embed = embed.timestamp(timestamp);
                }
            }

            let msg = serenity::CreateMessage::new().embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("⚔️  Posted CoC '{}' [{}] to {}", update.title, update.tag, channel_id_str);
                    total += 1;
                }
                Err(e) => {
                    error!("Failed to post CoC update to {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    Ok(total)
}
