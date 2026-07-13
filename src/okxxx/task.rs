use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::OkXxxClient;
use super::models::OkXxxVideo;

/// Single tick exposed for `/post` force-refresh.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    let client = OkXxxClient::new()?;
    let videos = client.fetch_videos(1).await?;
    post_videos(data, http, &videos, true).await
}

/// Background task — runs every 25 minutes, rotates through listing pages.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🔥 OK.XXX task started (scraping ok.xxx every 25 min)");

    let client = match OkXxxClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create OkXxxClient: {:#}", e);
            return;
        }
    };

    let mut tick: u64 = 0;

    loop {
        match client.fetch_for_tick(tick).await {
            Ok(videos) => {
                match post_videos(&data, &http, &videos, false).await {
                    Ok(n) if n > 0 => info!("🔥 OK.XXX: posted {} video(s) (page {})", n, (tick % 5) + 1),
                    Ok(_) => {}
                    Err(e) => error!("OK.XXX post error: {:#}", e),
                }
            }
            Err(e) => error!("OK.XXX fetch error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_okxxx(&data.db, 60).await {
            warn!("Could not prune seen_okxxx: {}", e);
        }

        tick += 1;
        tokio::time::sleep(Duration::from_secs(25 * 60)).await;
    }
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[OkXxxVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.okxxx_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.okxxx_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid okxxx_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for video in videos {
            // Dedup check
            if !force {
                match queries::is_okxxx_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                    Ok(true) => continue,
                    Err(e) => { error!("DB error checking seen_okxxx: {}", e); continue; }
                    _ => {}
                }
            }

            if let Err(e) = queries::mark_okxxx_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                error!("DB error marking okxxx seen: {}", e);
            }

            // Cap at 4 per tick per guild
            if posted_this_tick >= 4 {
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

            let play_url = format!(
                "{}/play?url={}&source=okxxx&title={}",
                data.config.public_url,
                crate::web::encode_hex(&video.url),
                url::form_urlencoded::byte_serialize(video.title.as_bytes()).collect::<String>()
            );

            let embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.url)
                .image(&video.thumbnail)
                .description(format!("🌐 **[Web Stream Player]({})**", play_url))
                .color(0xFF4500) // Deep orange-red
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🔥 OK.XXX{}{}",
                    duration_str, views_str
                )));

            let msg = serenity::CreateMessage::new()
                .content(&video.url)
                .embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🔥 OK.XXX: posted video {} to guild {}", video.video_id, cfg.guild_id);
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post okxxx video to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(600)).await;
        }
    }

    Ok(total)
}
