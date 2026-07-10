use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::{PornClient, PORN_SEARCHES};

/// Single tick exposed for `/admin force-refresh`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    tick(data, http).await
}

/// Background task — runs every 45 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🔞 Porn video task started (RedTube API — NaughtyAmerica, Brazzers, etc.)");

    let client = match PornClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create PornClient: {:#}", e);
            return;
        }
    };

    // Rotate through categories each tick
    let mut category_index = 0usize;

    loop {
        let search = PORN_SEARCHES[category_index % PORN_SEARCHES.len()];
        category_index += 1;

        match tick_with_search(&data, &http, &client, search).await {
            Ok(n) if n > 0 => info!("🔞 Posted {} porn video(s) from search \"{}\"", n, search),
            Ok(_) => {}
            Err(e) => error!("Porn video task error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_porn_videos(&data.db, 60).await {
            warn!("Could not prune seen_porn_videos: {}", e);
        }

        // Run every 45 minutes
        tokio::time::sleep(Duration::from_secs(45 * 60)).await;
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    let client = PornClient::new()?;
    // On manual force-refresh, fetch top rated for variety
    let videos = client.fetch_top_rated(10).await?;
    post_videos(data, http, &videos).await
}

async fn tick_with_search(
    data: &Data,
    http: &Arc<serenity::Http>,
    client: &PornClient,
    search: &str,
) -> Result<usize> {
    let videos = client.fetch_videos(search, 10).await?;
    post_videos(data, http, &videos).await
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[super::models::RedTubeVideo],
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.porn_video_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.porn_video_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid porn_video_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for video in videos {
            // Dedup
            match queries::is_porn_video_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                Ok(true) => continue,
                Err(e) => { error!("DB error checking seen_porn_videos: {}", e); continue; }
                _ => {}
            }

            if let Err(e) = queries::mark_porn_video_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                error!("DB error marking porn video seen: {}", e);
            }

            // Spam prevention — max 2 per tick per guild
            if posted_this_tick >= 2 {
                continue;
            }

            // Format tags (top 4 only)
            let tags: Vec<&str> = video.tags.iter().take(4).map(|t| t.tag_name.as_str()).collect();
            let tag_str = if tags.is_empty() { String::new() } else { format!(" • 🏷️ {}", tags.join(", ")) };

            // Parse views nicely
            let views_str = if video.views >= 1_000_000 {
                format!("{:.1}M", video.views as f64 / 1_000_000.0)
            } else if video.views >= 1_000 {
                format!("{:.0}K", video.views as f64 / 1_000.0)
            } else {
                video.views.to_string()
            };

            let embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.url)
                .image(&video.default_thumb)
                .color(0xFF1744) // Deep red
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🔞 RedTube • ⏱️ {} • 👁️ {} views • ⭐ {}%{}",
                    video.duration, views_str, video.rating.split('.').next().unwrap_or(&video.rating), tag_str
                )));

            let msg = serenity::CreateMessage::new().embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🔞 Posted porn video {} to guild {}", video.video_id, cfg.guild_id);
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post porn video to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    Ok(total)
}
