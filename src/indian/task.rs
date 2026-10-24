use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::{IndianClient, INDIAN_SEARCHES};
use super::models::IndianVideo;

/// Single tick exposed for `/admin force-refresh` and `/post`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = IndianClient::new()?;
    for &query in INDIAN_SEARCHES.iter().take(8) {
        for page in 1..=3 {
            let videos = match client.fetch_indian_videos(query, 10, page).await {
                Ok(v) if !v.is_empty() => v,
                _ => continue,
            };
            let posted = post_videos(data, http, &client, &videos, force).await?;
            if posted > 0 {
                return Ok(posted);
            }
        }
    }
    Ok(0)
}

/// Single tick for a specific query and page.
pub async fn run_with_query(
    data: &Data,
    http: &Arc<serenity::Http>,
    query: &str,
    page: u32,
    force: bool,
) -> Result<usize> {
    let client = IndianClient::new()?;
    tick_with_query(data, http, &client, query, page, force).await
}

/// Background task — runs every 20 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🇮🇳 Indian Viral MMS task started (every 20 min)");

    let client = match IndianClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create IndianClient: {:#}", e);
            return;
        }
    };

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Deterministic start offset based on 20-min intervals since epoch to prevent restart resets
    let mut tick_idx: usize = ((now_secs / 1200) % 100) as usize;

    loop {
        // Try up to 3 queries each tick until videos are successfully posted
        for offset in 0..3 {
            let query = INDIAN_SEARCHES[(tick_idx + offset) % INDIAN_SEARCHES.len()];
            let page = ((tick_idx * 2 + offset) % 10 + 1) as u32;

            match tick_with_query(&data, &http, &client, query, page, false).await {
                Ok(n) if n > 0 => {
                    info!("🇮🇳 Posted {} Indian MMS video(s) for query '{}' (page {})", n, query, page);
                    break;
                }
                Ok(_) => {}
                Err(e) => error!("Indian MMS task error for query '{}': {:#}", query, e),
            }
        }

        tick_idx = tick_idx.wrapping_add(1);

        if let Err(e) = queries::prune_old_seen_indian(&data.db, 90).await {
            warn!("Could not prune seen_indian: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(20 * 60)).await;
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = IndianClient::new()?;
    for &query in INDIAN_SEARCHES.iter().take(4) {
        let videos = match client.fetch_indian_videos(query, 10, 1).await {
            Ok(v) if !v.is_empty() => v,
            _ => continue,
        };
        let posted = post_videos(data, http, &client, &videos, force).await?;
        if posted > 0 {
            return Ok(posted);
        }
    }
    Ok(0)
}

async fn tick_with_query(
    data: &Data,
    http: &Arc<serenity::Http>,
    client: &IndianClient,
    query: &str,
    page: u32,
    force: bool,
) -> Result<usize> {
    let videos = client.fetch_indian_videos(query, 8, page).await?;
    post_videos(data, http, client, &videos, force).await
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    client: &IndianClient,
    videos: &[IndianVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.indian_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.indian_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid indian_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for video in videos {
            // Limit to 2 per tick per guild to conserve bandwidth
            if posted_this_tick >= 2 {
                break;
            }

            if !force {
                // Deduplicate via seen_indian table
                match queries::is_indian_seen(&data.db, &cfg.guild_id, &video.id).await {
                    Ok(true) => continue,
                    Err(e) => {
                        error!("DB error checking seen_indian: {}", e);
                        continue;
                    }
                    _ => {}
                }
            }

            let views_str = match &video.views {
                serde_json::Value::Number(n) => format_views(n.as_u64().unwrap_or(0)),
                serde_json::Value::String(s) => {
                    if s.ends_with('K') || s.ends_with('M') || s.ends_with('B') || s.ends_with('%') {
                        s.clone()
                    } else if let Ok(n) = s.parse::<u64>() {
                        format_views(n)
                    } else if !s.is_empty() {
                        s.clone()
                    } else {
                        "N/A".to_string()
                    }
                }
                _ => "N/A".to_string(),
            };

            let footer = format!(
                "🇮🇳 Instagram Viral MMS • ⏱️ {} • 👁️ {}",
                video.duration, views_str
            );

            let play_url = format!(
                "{}/play?url={}&source=indian&title={}",
                data.config.public_url,
                crate::web::encode_hex(&video.url),
                url::form_urlencoded::byte_serialize(video.title.as_bytes()).collect::<String>()
            );

            let mut embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&play_url)
                .description(format!("🌐 **[▶️ Watch in Web Stream Player]({})**", play_url))
                .color(0xFF9933) // Saffron / Indian Orange
                .footer(serenity::CreateEmbedFooter::new(footer));

            if !video.default_thumb.is_empty() {
                embed = embed.image(&video.default_thumb);
            }

            // Attempt to download MP4 if small enough (< 8MB) to send as native Discord video attachment
            let maybe_bytes = if let Some(mp4_url) = &video.direct_mp4 {
                match client.download_bytes(mp4_url).await {
                    Ok(b) if b.len() <= 8 * 1024 * 1024 => Some(b),
                    _ => None,
                }
            } else if let Ok(mp4_url) = client.get_mp4_url(&video.url).await {
                if !mp4_url.contains(".m3u8") && mp4_url.starts_with("http") {
                    match client.download_bytes(&mp4_url).await {
                        Ok(b) if b.len() <= 8 * 1024 * 1024 => Some(b),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let send_result = if let Some(bytes) = maybe_bytes {
                let filename = format!("{}.mp4", video.id);
                let attachment = serenity::CreateAttachment::bytes(bytes, filename);
                let msg = serenity::CreateMessage::new()
                    .embed(embed.clone())
                    .add_file(attachment);
                channel.send_message(http, msg).await
            } else {
                let msg = serenity::CreateMessage::new()
                    .embed(embed.clone());
                channel.send_message(http, msg).await
            };

            // If attachment upload failed (e.g. Discord 413 Payload Too Large), fallback to embed
            let final_result = match send_result {
                Ok(m) => Ok(m),
                Err(e) => {
                    warn!("Attachment upload failed for Indian video {}, falling back to embed: {:#}", video.id, e);
                    let msg = serenity::CreateMessage::new()
                        .embed(embed);
                    channel.send_message(http, msg).await
                }
            };

            match final_result {
                Ok(_) => {
                    info!("🇮🇳 Posted Indian MMS video {} to guild {}", video.id, cfg.guild_id);
                    // Mark as seen only after successful delivery
                    if let Err(e) = queries::mark_indian_seen(&data.db, &cfg.guild_id, &video.id).await {
                        error!("DB error marking indian seen: {}", e);
                    }
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post Indian MMS video to channel {}: {}", channel_id_str, e);
                }
            }
        }
    }

    Ok(total)
}

fn format_views(v: u64) -> String {
    if v >= 1_000_000 {
        format!("{:.1}M", v as f64 / 1_000_000.0)
    } else if v >= 1_000 {
        format!("{:.1}K", v as f64 / 1_000.0)
    } else {
        v.to_string()
    }
}
