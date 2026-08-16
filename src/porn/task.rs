use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::{PornClient, BeegTvClient, PORN_SEARCHES};

/// Single tick exposed for `/admin force-refresh`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    let client = PornClient::new()?;
    let videos = client.fetch_top_rated(10).await?;
    let n = post_redtube_videos(data, http, &videos, true).await?;

    // Also pull a batch from beeg.tv on manual refresh
    let beegtv = BeegTvClient::new()?;
    match beegtv.fetch_latest(10).await {
        Ok(bv) => {
            let n2 = post_beegtv_videos(data, http, &bv, true).await?;
            Ok(n + n2)
        }
        Err(e) => {
            warn!("Beeg.tv fetch failed during force-refresh: {:#}", e);
            Ok(n)
        }
    }
}

/// Background task — runs every 20 minutes, alternating sources.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🔞 Porn video task started (RedTube + Beeg.tv)");

    let redtube = match PornClient::new() {
        Ok(c) => c,
        Err(e) => { error!("Failed to create PornClient: {:#}", e); return; }
    };
    let beegtv = match BeegTvClient::new() {
        Ok(c) => c,
        Err(e) => { error!("Failed to create BeegTvClient: {:#}", e); return; }
    };

    let mut tick_count = 0usize;
    let mut category_index = 0usize;

    loop {
        // Alternate: even ticks → RedTube, odd ticks → Beeg.tv
        if tick_count % 2 == 0 {
            let search = PORN_SEARCHES[category_index % PORN_SEARCHES.len()];
            category_index += 1;
            match redtube.fetch_videos(search, 10).await {
                Ok(videos) => {
                    match post_redtube_videos(&data, &http, &videos, false).await {
                        Ok(n) if n > 0 => info!("🔞 Posted {} RedTube video(s) [{}]", n, search),
                        Ok(_) => {}
                        Err(e) => error!("RedTube post error: {:#}", e),
                    }
                }
                Err(e) => error!("RedTube fetch error ({}): {:#}", search, e),
            }
        } else {
            match beegtv.fetch_latest(10).await {
                Ok(videos) => {
                    match post_beegtv_videos(&data, &http, &videos, false).await {
                        Ok(n) if n > 0 => info!("🔞 Posted {} Beeg.tv video(s)", n),
                        Ok(_) => {}
                        Err(e) => error!("Beeg.tv post error: {:#}", e),
                    }
                }
                Err(e) => error!("Beeg.tv fetch error: {:#}", e),
            }
        }

        tick_count += 1;

        if let Err(e) = queries::prune_old_seen_porn_videos(&data.db, 60).await {
            warn!("Could not prune seen_porn_videos: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(20 * 60)).await;
    }
}

async fn post_redtube_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[super::models::RedTubeVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs.into_iter().filter(|c| c.porn_video_channel_id.is_some()).collect();
    if relevant.is_empty() { return Ok(0); }

    let mut total = 0usize;
    for cfg in relevant {
        let channel_id_str = cfg.porn_video_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => { warn!("Invalid porn_video_channel_id for guild {}", cfg.guild_id); continue; }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted = 0usize;

        for video in videos {
            if !force {
                match queries::is_porn_video_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                    Ok(true) => continue,
                    Err(e) => { error!("DB error: {}", e); continue; }
                    _ => {}
                }
            }
            if let Err(e) = queries::mark_porn_video_seen(&data.db, &cfg.guild_id, &video.video_id).await {
                error!("DB mark error: {}", e);
            }
            if posted >= 5 { continue; }

            let tags: Vec<&str> = video.tags.iter().take(4).map(|t| t.tag_name.as_str()).collect();
            let tag_str = if tags.is_empty() { String::new() } else { format!(" • 🏷️ {}", tags.join(", ")) };
            let views_str = if video.views >= 1_000_000 {
                format!("{:.1}M", video.views as f64 / 1_000_000.0)
            } else if video.views >= 1_000 {
                format!("{:.0}K", video.views as f64 / 1_000.0)
            } else { video.views.to_string() };

            let embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.url)
                .image(&video.default_thumb)
                .color(0xFF1744)
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🔞 RedTube • ⏱️ {} • 👁️ {} views • ⭐ {}%{}",
                    video.duration, views_str,
                    video.rating.split('.').next().unwrap_or(&video.rating),
                    tag_str
                )));

            let msg = serenity::CreateMessage::new().content(&video.url).embed(embed);
            match channel.send_message(http, msg).await {
                Ok(_) => { total += 1; posted += 1; }
                Err(e) => error!("Failed to post RedTube video: {}", e),
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    Ok(total)
}

async fn post_beegtv_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[super::models::BeegTvVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs.into_iter().filter(|c| c.porn_video_channel_id.is_some()).collect();
    if relevant.is_empty() { return Ok(0); }

    let mut total = 0usize;
    for cfg in relevant {
        let channel_id_str = cfg.porn_video_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => { warn!("Invalid porn_video_channel_id for guild {}", cfg.guild_id); continue; }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);
        let mut posted = 0usize;

        for video in videos {
            // Use "beegtv_" prefix in seen table to namespace from RedTube IDs
            let seen_key = format!("beegtv_{}", video.video_id);
            if !force {
                match queries::is_porn_video_seen(&data.db, &cfg.guild_id, &seen_key).await {
                    Ok(true) => continue,
                    Err(e) => { error!("DB error: {}", e); continue; }
                    _ => {}
                }
            }
            if let Err(e) = queries::mark_porn_video_seen(&data.db, &cfg.guild_id, &seen_key).await {
                error!("DB mark error: {}", e);
            }
            if posted >= 5 { continue; }

            let embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.url)
                .image(&video.thumbnail)
                .color(0xFF6D00) // Orange — distinct from RedTube's red
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🔞 Beeg.tv • ⏱️ {} • 🔗 Watch full video",
                    video.duration
                )));

            let msg = serenity::CreateMessage::new().content(&video.url).embed(embed);
            match channel.send_message(http, msg).await {
                Ok(_) => { total += 1; posted += 1; }
                Err(e) => error!("Failed to post Beeg.tv video: {}", e),
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    Ok(total)
}
