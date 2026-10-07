use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn, debug};

use crate::data::Data;
use crate::db::queries;
use super::client::PornClipsClient;

/// Single tick exposed for `/admin force-refresh` and `/post`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = PornClipsClient::new().await?;
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let clips = client.fetch_for_tick(tick).await?;
    post_clips(data, http, &client, &clips, force).await
}

/// Background task — runs every 10 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎬 Porn Clips task started (every 10 min — RedGIFs short clips)");

    let client = match PornClipsClient::new().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create PornClipsClient: {:#}", e);
            return;
        }
    };

    let mut tick = 0u64;

    loop {
        match client.fetch_for_tick(tick).await {
            Ok(clips) => {
                match post_clips(&data, &http, &client, &clips, false).await {
                    Ok(n) if n > 0 => info!("🎬 Posted {} Porn Clip(s) for tick {}", n, tick),
                    Ok(_) => {}
                    Err(e) => error!("Porn Clips task error: {:#}", e),
                }
            }
            Err(e) => error!("Failed to fetch Porn Clips for tick {}: {:#}", tick, e),
        }

        tick += 1;

        if let Err(e) = queries::prune_old_seen_pornclips(&data.db, 90).await {
            warn!("Could not prune seen_pornclips: {}", e);
        }

        // Run every 10 minutes
        tokio::time::sleep(Duration::from_secs(10 * 60)).await;
    }
}

async fn post_clips(
    data: &Data,
    http: &Arc<serenity::Http>,
    client: &PornClipsClient,
    clips: &[super::models::RedGifsGif],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.pornclips_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.pornclips_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid pornclips_channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted_this_tick = 0usize;

        for clip in clips {
            // Cap at 3 clips per 10-minute tick per guild
            if posted_this_tick >= 3 {
                break;
            }

            let item_id = clip.id.trim().to_lowercase();

            // Deduplication check: check BEFORE downloading
            if !force {
                match queries::is_pornclips_seen(&data.db, &cfg.guild_id, &item_id).await {
                    Ok(true) => {
                        debug!("🎬 Clip {} already seen in guild {}, skipping", item_id, cfg.guild_id);
                        continue;
                    }
                    Err(e) => {
                        error!("DB error checking seen_pornclips: {}", e);
                        continue;
                    }
                    _ => {}
                }
            }

            let sd_url = match clip.urls.as_ref().and_then(|u| u.sd.as_deref()) {
                Some(u) if !u.is_empty() => u,
                _ => continue,
            };

            // Attempt to download MP4 if small enough (< 8MB) to send as file attachment.
            // If download fails, file is too big, or upload fails, we seamlessly post the RedGIFs URL
            // which Discord embeds natively with its built-in video player!
            let maybe_bytes = match client.download_bytes(sd_url).await {
                Ok(b) if b.len() <= 8 * 1024 * 1024 => Some(b),
                _ => None,
            };

            // Format duration as mm:ss
            let dur_secs = clip.duration.unwrap_or(15.0) as u64;
            let dur_str = format!("{}:{:02}", dur_secs / 60, dur_secs % 60);

            let views_str = format_views(clip.views.unwrap_or(0));
            let author_name = clip.user_name.as_deref().unwrap_or("Anonymous");

            let tags_preview = clip.tags.iter().take(3)
                .map(|t| t.as_str())
                .collect::<Vec<_>>()
                .join(" • ");

            let title = if clip.tags.is_empty() {
                format!("🔥 Clip by {}", author_name)
            } else {
                format!("🔥 {}", tags_preview)
            };

            let footer = format!(
                "🎬 Porn Clips • ⏱️ {} • 👁️ {} views • by {}",
                dur_str, views_str, author_name
            );

            let page_url = format!("https://www.redgifs.com/watch/{}", clip.id);

            let embed = serenity::CreateEmbed::new()
                .title(&title)
                .url(&page_url)
                .color(0xE91E63)
                .footer(serenity::CreateEmbedFooter::new(footer));

            let send_result = if let Some(bytes) = maybe_bytes {
                let filename = format!("{}.mp4", clip.id);
                let attachment = serenity::CreateAttachment::bytes(bytes, filename);
                let msg = serenity::CreateMessage::new()
                    .embed(embed.clone())
                    .add_file(attachment);
                channel.send_message(http, msg).await
            } else {
                let msg = serenity::CreateMessage::new()
                    .content(&page_url)
                    .embed(embed.clone());
                channel.send_message(http, msg).await
            };

            // If attachment upload failed (e.g. Discord 413 Payload Too Large), retry immediately with native URL!
            let final_result = match send_result {
                Ok(m) => Ok(m),
                Err(e) => {
                    warn!("Attachment upload failed for clip {}, falling back to native URL embed: {:#}", item_id, e);
                    let fallback_msg = serenity::CreateMessage::new()
                        .content(&page_url)
                        .embed(embed);
                    channel.send_message(http, fallback_msg).await
                }
            };

            match final_result {
                Ok(_) => {
                    info!("🎬 Posted Porn Clip {} to guild {}", item_id, cfg.guild_id);
                    if let Err(e) = queries::mark_pornclips_seen(&data.db, &cfg.guild_id, &item_id).await {
                        error!("DB error marking pornclips seen: {}", e);
                    }
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post Porn Clip to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    Ok(total)
}

fn format_views(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
