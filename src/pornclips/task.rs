use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

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
    post_clips(data, http, &clips, force).await
}

/// Background task — runs every 15 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎬 Porn Clips task started (RedGIFs short clips)");

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
                match post_clips(&data, &http, &clips, false).await {
                    Ok(n) if n > 0 => info!("🎬 Posted {} Porn Clip(s) for tick {}", n, tick),
                    Ok(_) => {}
                    Err(e) => error!("Porn Clips task error: {:#}", e),
                }
            }
            Err(e) => error!("Failed to fetch Porn Clips for tick {}: {:#}", tick, e),
        }

        tick += 1;

        if let Err(e) = queries::prune_old_seen_pornclips(&data.db, 30).await {
            warn!("Could not prune seen_pornclips: {}", e);
        }

        // Run every 15 minutes
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}

async fn post_clips(
    data: &Data,
    http: &Arc<serenity::Http>,
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
            if !force {
                // Deduplicate via seen_pornclips table
                match queries::is_pornclips_seen(&data.db, &cfg.guild_id, &clip.id).await {
                    Ok(true) => continue,
                    Err(e) => {
                        error!("DB error checking seen_pornclips: {}", e);
                        continue;
                    }
                    _ => {}
                }
            }

            if let Err(e) = queries::mark_pornclips_seen(&data.db, &cfg.guild_id, &clip.id).await {
                error!("DB error marking pornclips seen: {}", e);
            }

            // Limit to 5 per tick per guild
            if posted_this_tick >= 5 {
                continue;
            }

            // Format duration as mm:ss
            let dur_secs = clip.duration as u64;
            let dur_str = format!("{}:{:02}", dur_secs / 60, dur_secs % 60);

            // Format views
            let views_str = format_views(clip.views);

            let footer = format!(
                "🎬 Porn Clips • ⏱️ {} • 👁️ {} views",
                dur_str, views_str
            );

            let title = if clip.tags.is_empty() {
                format!("🔥 Clip by {}", clip.user_name)
            } else {
                format!("🔥 {} — {}", clip.tags.first().unwrap_or(&String::new()), clip.user_name)
            };

            let page_url = format!("https://www.redgifs.com/watch/{}", clip.id);

            let embed = serenity::CreateEmbed::new()
                .title(&title)
                .url(&page_url)
                .color(0xE91E63);

            // If we have a poster, add it as the embed image
            let embed = if let Some(poster) = &clip.urls.poster {
                embed.image(poster)
            } else {
                embed
            };
            let embed = embed.footer(serenity::CreateEmbedFooter::new(footer));

            // Post SD MP4 directly — Discord renders inline video player
            let sd_url = clip.urls.sd.as_deref().unwrap_or("");
            let content = sd_url.to_string();
            let msg = serenity::CreateMessage::new()
                .content(&content)
                .embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🎬 Posted Porn Clip {} to guild {}", clip.id, cfg.guild_id);
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
