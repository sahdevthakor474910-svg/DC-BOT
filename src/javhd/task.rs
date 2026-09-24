use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::JavhdClient;
use super::models::JavhdVideo;

/// Single tick exposed for `/post` force-refresh.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = JavhdClient::new()?;
    let videos = client.fetch_for_tick(0).await?;
    post_videos(data, http, &videos, force).await
}

/// Background task — runs every 20 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎌 JAVHD task started (every 20 min — javhd.com API)");

    let client = match JavhdClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create JavhdClient: {:#}", e);
            return;
        }
    };

    let mut tick: u64 = 0;

    loop {
        match client.fetch_for_tick(tick).await {
            Ok(videos) => {
                match post_videos(&data, &http, &videos, false).await {
                    Ok(n) if n > 0 => info!("🎌 JAVHD: posted {} video(s) (tick {})", n, tick),
                    Ok(_) => {}
                    Err(e) => error!("JAVHD post error: {:#}", e),
                }
            }
            Err(e) => error!("JAVHD fetch error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_javhd(&data.db, 30).await {
            warn!("Could not prune seen_javhd: {}", e);
        }

        tick += 1;
        tokio::time::sleep(Duration::from_secs(20 * 60)).await;
    }
}

fn format_views(views: u64) -> String {
    if views >= 1_000_000 {
        format!("{:.1}M+", (views as f64) / 1_000_000.0)
    } else if views >= 1_000 {
        format!("{:.1}K+", (views as f64) / 1_000.0)
    } else {
        views.to_string()
    }
}

fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.as_bytes() {
        match *b as char {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[JavhdVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.javhd_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;
    let public_url = std::env::var("PUBLIC_URL").unwrap_or_else(|_| String::new());

    for cfg in relevant {
        let channel_id_str = cfg.javhd_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid javhd_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for video in videos {
            // Dedup check
            if !force {
                match queries::is_javhd_seen(&data.db, &cfg.guild_id, &video.id).await {
                    Ok(true) => continue,
                    Err(e) => { error!("DB error checking seen_javhd: {}", e); continue; }
                    _ => {}
                }
            }

            if let Err(e) = queries::mark_javhd_seen(&data.db, &cfg.guild_id, &video.id).await {
                error!("DB error marking javhd seen: {}", e);
            }

            // Cap at 3 per tick per guild
            if posted_this_tick >= 3 {
                break;
            }

            let views_str = if video.views == 0 {
                String::new()
            } else {
                format!(" • 👁️ {}", format_views(video.views))
            };

            let duration_str = if video.duration == "?" {
                String::new()
            } else {
                format!(" • ⏱️ {}", video.duration)
            };

            let mut embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .color(0xE91E63) // Pink
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🎌 JAVHD{}{}",
                    duration_str, views_str
                )));

            if !video.thumb_url.is_empty() {
                embed = embed.image(&video.thumb_url);
            }

            if !public_url.is_empty() {
                let hex_encoded_url = crate::web::encode_hex(&video.page_url);
                let encoded_title = url_encode(&video.title);
                embed = embed.url(format!("{}/play?url={}&source=javhd&title={}", public_url, hex_encoded_url, encoded_title));
            } else {
                embed = embed.url(&video.page_url);
            }

            // Post the preview MP4 URL as content so Discord inline player works
            let msg = serenity::CreateMessage::new()
                .content(&video.preview_mp4)
                .embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🎌 JAVHD: posted video {} to guild {}", video.id, cfg.guild_id);
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post javhd video to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(600)).await;
        }
    }

    Ok(total)
}
