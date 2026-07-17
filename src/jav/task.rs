use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::{EpornerClient, JAV_SEARCHES};

/// Single tick exposed for `/admin force-refresh` and `/post`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    tick(data, http, force).await
}

/// Background task — runs every 15 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎌 JAV task started (eporner.com — direct MP4 playback)");

    let client = match EpornerClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create EpornerClient: {:#}", e);
            return;
        }
    };

    // Rotate through JAV search queries each tick
    let mut search_index = 0usize;

    loop {
        let query = JAV_SEARCHES[search_index % JAV_SEARCHES.len()];
        search_index += 1;

        match tick_with_query(&data, &http, &client, query, false).await {
            Ok(n) if n > 0 => info!("🎌 Posted {} JAV video(s) for query \"{}\"", n, query),
            Ok(_) => {}
            Err(e) => error!("JAV task error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_jav(&data.db, 30).await {
            warn!("Could not prune seen_jav: {}", e);
        }

        // Run every 15 minutes
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>, force: bool) -> Result<usize> {
    let client = EpornerClient::new()?;
    let videos = client.fetch_jav_videos("japanese uncensored", 8).await?;
    post_videos(data, http, &videos, force).await
}

async fn tick_with_query(
    data: &Data,
    http: &Arc<serenity::Http>,
    client: &EpornerClient,
    query: &str,
    force: bool,
) -> Result<usize> {
    let videos = client.fetch_jav_videos(query, 8).await?;
    post_videos(data, http, &videos, force).await
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
    videos: &[super::models::EpornerVideo],
    force: bool,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.jav_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
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
        let mut posted_this_tick = 0usize;

        for video in videos {
            if !force {
                // Deduplicate via seen_jav table
                match queries::is_jav_seen(&data.db, &cfg.guild_id, &video.id).await {
                    Ok(true) => continue,
                    Err(e) => {
                        error!("DB error checking seen_jav: {}", e);
                        continue;
                    }
                    _ => {}
                }
            }

            if let Err(e) = queries::mark_jav_seen(&data.db, &cfg.guild_id, &video.id).await {
                error!("DB error marking jav seen: {}", e);
            }

            // Limit to 5 per tick per guild
            if posted_this_tick >= 5 {
                continue;
            }

            // Format views nicely
            let views_str = format_views(&video.views);

            // Build the footer
            let footer = format!(
                "🎌 eporner • ⏱️ {} • 👁️ {} views",
                video.duration, views_str
            );

            let play_url = format!(
                "{}/play?url={}&source=jav&title={}",
                data.config.public_url,
                crate::web::encode_hex(&video.page_url),
                url::form_urlencoded::byte_serialize(video.title.as_bytes()).collect::<String>()
            );

            let embed = serenity::CreateEmbed::new()
                .title(&video.title)
                .url(&video.page_url)
                .description(format!("🌐 **[Web Stream Player]({})**", play_url))
                .color(0xFF3366) // Hot pink for JAV
                .footer(serenity::CreateEmbedFooter::new(footer));

            // Post the direct MP4 URL as message content so Discord renders
            // an inline video player with audio. The embed provides title + link.
            let content = format!("🎥 **{}**\n{}", video.title, video.mp4_url);
            let mut msg_builder = serenity::CreateMessage::new().content(&content);

            // Only attach the embed if we have a thumbnail
            if !video.thumb_url.is_empty() {
                msg_builder = msg_builder.embed(embed.image(&video.thumb_url));
            } else {
                msg_builder = msg_builder.embed(embed);
            }

            match channel.send_message(http, msg_builder).await {
                Ok(_) => {
                    info!("🎌 Posted JAV video {} to guild {}", video.id, cfg.guild_id);
                    total += 1;
                    posted_this_tick += 1;
                }
                Err(e) => {
                    error!("Failed to post JAV video to channel {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    Ok(total)
}

fn format_views(views: &str) -> String {
    let n: u64 = views.parse().unwrap_or(0);
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        views.to_string()
    }
}
