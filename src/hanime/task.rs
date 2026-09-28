use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::HentaiClient;
use super::models::HentaiVideo;

/// Single tick exposed for `/post` force-refresh.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = HentaiClient::new()?;
    // Use unix seconds to rotate starting page on manual /post
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let videos = client.fetch_with_mp4(tick).await?;
    post_videos(data, http, &videos, force).await
}

/// Background task — runs every 20 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🔮 Hentai video task started (every 20 min — hentaigasm.com with direct MP4)");

    let client = match HentaiClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create HentaiClient: {:#}", e);
            return;
        }
    };

    let mut tick: u64 = 0;

    loop {
        match client.fetch_with_mp4(tick).await {
            Ok(videos) => {
                info!("🔮 Hentai: fetched {} video(s) with MP4 (tick {})", videos.len(), tick);
                match post_videos(&data, &http, &videos, false).await {
                    Ok(n) if n > 0 => info!("🔮 Hentai: posted {} video(s) (tick {})", n, tick),
                    Ok(_) => {}
                    Err(e) => error!("Hentai post error: {:#}", e),
                }
            }
            Err(e) => error!("Hentai fetch error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_hanime(&data.db, 30).await {
            warn!("Could not prune seen_hanime: {}", e);
        }

        tick += 1;
        tokio::time::sleep(Duration::from_secs(20 * 60)).await;
    }
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[HentaiVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.hanime_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.hanime_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid hanime_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for video in videos {
            // Cap at 3 per tick per guild
            if posted_this_tick >= 3 {
                break;
            }

            // Dedup check (skip if not forced and already seen)
            if !force {
                match queries::is_hanime_seen(&data.db, &cfg.guild_id, &video.id).await {
                    Ok(true) => continue,
                    Err(e) => {
                        error!("DB error checking seen_hanime: {}", e);
                        continue;
                    }
                    _ => {}
                }
            }

            // Format footer
            let views_str = if video.views.is_empty() {
                String::new()
            } else {
                format!(" • 👁️ {} views", video.views)
            };
            let likes_str = if video.likes.is_empty() {
                String::new()
            } else {
                format!(" • 👍 {}", video.likes)
            };
            let footer_text = format!("🔮 Hentai{}{}", views_str, likes_str);

            let play_url = format!(
                "{}/play?url={}&source=hentai&title={}",
                data.config.public_url,
                crate::web::encode_hex(&video.url),
                url::form_urlencoded::byte_serialize(video.title.as_bytes()).collect::<String>()
            );

            let mut embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.url)
                .description(format!("🌐 **[Web Stream Player]({})**", play_url))
                .color(0x9B59B6)
                .footer(serenity::CreateEmbedFooter::new(footer_text));

            // Only attach image if valid URL without unencoded spaces
            if !video.thumbnail.is_empty() && video.thumbnail.starts_with("http") {
                embed = embed.image(&video.thumbnail);
            }

            // Post direct MP4 URL as content so Discord automatically creates the native video player
            let content = if !video.mp4_url.is_empty() {
                format!("🎥 **{}**\n{}", video.title, video.mp4_url)
            } else {
                format!("🎥 **{}**\n{}", video.title, video.url)
            };

            let msg = serenity::CreateMessage::new()
                .content(&content)
                .embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🔮 Hentai: posted video {} to guild {}", video.id, cfg.guild_id);
                    if let Err(e) = queries::mark_hanime_seen(&data.db, &cfg.guild_id, &video.id).await {
                        error!("DB error marking hanime seen: {}", e);
                    }
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post hentai video to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(600)).await;
        }
    }

    Ok(total)
}
