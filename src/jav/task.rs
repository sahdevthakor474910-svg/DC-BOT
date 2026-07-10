use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client;

/// Single tick — exposed for `/admin force-refresh`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    tick(data, http).await
}

/// Background task — runs every 2 hours.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎌 JAV task started");

    loop {
        match tick(&data, &http).await {
            Ok(n) if n > 0 => info!("🎌 Posted {} JAV title(s)", n),
            Ok(_) => {}
            Err(e) => error!("JAV task error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_jav(&data.db, 30).await {
            warn!("Could not prune seen_jav: {}", e);
        }

        // Post every 2 hours — JAV releases are slower-paced than memes
        tokio::time::sleep(Duration::from_secs(2 * 60 * 60)).await;
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.jav_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let titles = client::fetch_all(&data.http_client).await;
    if titles.is_empty() {
        warn!("JAV fetch returned 0 titles");
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.jav_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid jav_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);

        for title in &titles {
            // Dedup check
            if queries::is_jav_seen(&data.db, &cfg.guild_id, &title.content_id).await? {
                continue;
            }

            queries::mark_jav_seen(&data.db, &cfg.guild_id, &title.content_id).await?;

            // Build the embed
            let label = if title.is_popular {
                "🔥 Popular JAV"
            } else {
                "🆕 Latest JAV Release"
            };

            let actress_str = if title.actresses.is_empty() {
                "Unknown".to_string()
            } else {
                title.actresses.join(", ")
            };

            let mut embed = serenity::CreateEmbed::new()
                .title(&title.title)
                .url(&title.url)
                .color(0xFF3366)
                .field("👩 Actress", &actress_str, true)
                .field(
                    "🏢 Studio",
                    title.studio.as_deref().unwrap_or("Unknown"),
                    true,
                )
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "{} • R18.dev",
                    label
                )));

            if let Some(date) = &title.release_date {
                embed = embed.field("📅 Released", date, true);
            }

            if let Some(cover) = &title.cover_url {
                embed = embed.image(cover);
            }

            let msg = serenity::CreateMessage::new().embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!(
                        "🎌 Posted JAV '{}' to guild {}",
                        title.title, cfg.guild_id
                    );
                    total += 1;
                }
                Err(e) => {
                    error!(
                        "Failed to post JAV title to channel {}: {}",
                        channel_id_str, e
                    );
                }
            }

            // Small delay between posts to avoid rate-limiting
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    }

    Ok(total)
}
