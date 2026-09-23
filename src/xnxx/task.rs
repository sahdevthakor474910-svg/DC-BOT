use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::XnxxClient;
use super::models::XnxxVideo;

/// Single tick exposed for `/post` force-refresh.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = XnxxClient::new()?;
    let videos = client.fetch_videos(0).await?;
    post_videos(data, http, &videos, force).await
}

/// Background task — runs every 30 minutes, rotates through listing pages.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🔥 XNXX task started (scraping xnxx.com/best/week every 30 min)");

    let client = match XnxxClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create XnxxClient: {:#}", e);
            return;
        }
    };

    let mut tick: u64 = 0;

    loop {
        match client.fetch_for_tick(tick).await {
            Ok(videos) => {
                match post_videos(&data, &http, &videos, false).await {
                    Ok(n) if n > 0 => info!("🔥 XNXX: posted {} video(s) (page {})", n, tick % 5),
                    Ok(_) => {}
                    Err(e) => error!("XNXX post error: {:#}", e),
                }
            }
            Err(e) => error!("XNXX fetch error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_xnxx(&data.db, 60).await {
            warn!("Could not prune seen_xnxx: {}", e);
        }

        tick += 1;
        tokio::time::sleep(Duration::from_secs(30 * 60)).await;
    }
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[XnxxVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.xnxx_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.xnxx_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid xnxx_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for video in videos {
            // Dedup check
            if !force {
                match queries::is_xnxx_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                    Ok(true) => continue,
                    Err(e) => { error!("DB error checking seen_xnxx: {}", e); continue; }
                    _ => {}
                }
            }

            if let Err(e) = queries::mark_xnxx_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                error!("DB error marking xnxx seen: {}", e);
            }

            // Cap at 3 per tick per guild
            if posted_this_tick >= 3 {
                break;
            }

            let views_str = if video.views.is_empty() {
                String::new()
            } else {
                format!(" • 👁️ {} views", video.views)
            };

            let duration_str = if video.duration.is_empty() {
                String::new()
            } else {
                format!(" • ⏱️ {}", video.duration)
            };

            let embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.url)
                .image(&video.thumbnail)
                .color(0xFF6B35) // XNXX orange
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🔥 XNXX{}{}",
                    duration_str, views_str
                )));

            let msg = serenity::CreateMessage::new()
                .content(&video.url)
                .embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🔥 XNXX: posted video {} to guild {}", video.video_id, cfg.guild_id);
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post xnxx video to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(600)).await;
        }
    }

    Ok(total)
}
