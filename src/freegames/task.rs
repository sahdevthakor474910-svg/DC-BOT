use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::aggregator;

/// Single tick for `/admin force-refresh`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    tick(data, http, true).await
}

/// Background task — runs every 15 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎁 Free-games task started");

    loop {
        match tick(&data, &http, false).await {
            Ok(n) if n > 0 => info!("🎁 Posted {} free-game alert(s)", n),
            Ok(_) => {}
            Err(e) => error!("Free-games task error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_giveaways(&data.db, 60).await {
            warn!("Could not prune seen_giveaways: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}

/// Store-specific embed accent colour.
fn store_colour(store: &str) -> u32 {
    match store {
        "Epic Games" => 0x2D2D2D,   // Epic dark
        "Steam"      => 0x1B2838,   // Steam dark blue
        "GOG"        => 0xA62AA2,   // GOG purple
        "Ubisoft"    => 0x0070FF,   // Ubisoft blue
        "Itch.io"    => 0xFF2449,   // Itch.io red
        "IndieGala"  => 0xE94D2E,   // IndieGala orange-red
        "Stove"      => 0x00C896,   // Stove teal
        "DRM-Free"   => 0x27AE60,   // generic green
        _            => 0x5865F2,   // Discord blurple fallback
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.free_games_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let games = aggregator::fetch_all(&data.http_client).await;
    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.free_games_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid free_games channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);

        for game in &games {
            if !force {
                if queries::is_giveaway_seen(&data.db, &cfg.guild_id, &game.id).await? {
                    continue;
                }
            }
            queries::mark_giveaway_seen(&data.db, &cfg.guild_id, &game.id).await?;

            // Build embed
            let mut embed = serenity::CreateEmbed::new()
                .title(format!("🎮 {} — FREE!", game.title))
                .url(&game.url)
                .color(store_colour(&game.store))
                .footer(serenity::CreateEmbedFooter::new(&game.claim_instructions));

            // Description
            let mut desc_parts: Vec<String> = Vec::new();
            if let Some(orig) = &game.original_price {
                desc_parts.push(format!("~~{}~~ → **FREE**", orig));
            } else {
                desc_parts.push("**FREE** right now!".to_string());
            }
            if let Some(d) = &game.description {
                desc_parts.push(format!("\n*{}*", d));
            }
            if !desc_parts.is_empty() {
                embed = embed.description(desc_parts.join("\n"));
            }

            // Thumbnail
            if let Some(thumb) = &game.thumbnail_url {
                embed = embed.thumbnail(thumb);
            }

            // Expiry timestamp
            if let Some(end) = game.end_date {
                embed = embed.field(
                    "⏰ Offer Ends",
                    format!("<t:{}:R>", end.timestamp()),
                    true,
                );
            }

            // Store field
            embed = embed.field("🏪 Store", &game.store, true);

            let msg = serenity::CreateMessage::new().embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🎁 Posted free game '{}' to {}", game.title, channel_id_str);
                    total += 1;
                }
                Err(e) => {
                    error!("Failed to post free game to {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    Ok(total)
}
