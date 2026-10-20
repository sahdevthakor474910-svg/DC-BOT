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
    tick(data, http, force).await
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
    info!("🇮🇳 Indian Viral MMS task started (every 20 min — RedTube API)");

    let client = match IndianClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create IndianClient: {:#}", e);
            return;
        }
    };

    let mut tick_idx: usize = 0;

    loop {
        let query = INDIAN_SEARCHES[tick_idx % INDIAN_SEARCHES.len()];
        let page = ((tick_idx / INDIAN_SEARCHES.len()) % 5 + 1) as u32;

        match tick_with_query(&data, &http, &client, query, page, false).await {
            Ok(n) if n > 0 => info!("🇮🇳 Posted {} Indian MMS video(s) for query '{}'", n, query),
            Ok(_) => {}
            Err(e) => error!("Indian MMS task error: {:#}", e),
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
    let videos = client.fetch_indian_videos("indian viral mms", 8, 1).await?;
    post_videos(data, http, &videos, force).await
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
    post_videos(data, http, &videos, force).await
}

async fn post_videos(
    data: &Data,
    http: &Arc<serenity::Http>,
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

            if let Err(e) = queries::mark_indian_seen(&data.db, &cfg.guild_id, &video.id).await {
                error!("DB error marking indian seen: {}", e);
            }

            // Limit to 2 per tick per guild to conserve bandwidth
            if posted_this_tick >= 2 {
                break;
            }

            let views_val = match &video.views {
                serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
                serde_json::Value::String(s) => s.parse().unwrap_or(0),
                _ => 0,
            };
            let views_str = format_views(views_val);

            let footer = format!(
                "🇮🇳 Indian Viral MMS • ⏱️ {} • 👁️ {} views",
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
                .url(&video.url)
                .description(format!("🌐 **[▶️ Watch in Web Stream Player]({})**", play_url))
                .color(0xFF9933) // Saffron / Indian Orange
                .footer(serenity::CreateEmbedFooter::new(footer));

            if !video.default_thumb.is_empty() {
                embed = embed.image(&video.default_thumb);
            }

            let content = format!("🇮🇳 **[{}]({})**", video.title, video.url);
            let msg = serenity::CreateMessage::new()
                .content(&content)
                .embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("🇮🇳 Posted Indian MMS video {} to guild {}", video.id, cfg.guild_id);
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
