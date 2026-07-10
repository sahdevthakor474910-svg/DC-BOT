use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::fetcher;

/// Single tick exposed for `/admin force-refresh`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    tick(data, http).await
}

/// Background task — runs every 5 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("📰 Gaming-news task started");

    loop {
        match tick(&data, &http).await {
            Ok(n) if n > 0 => info!("📰 Posted {} gaming news article(s)", n),
            Ok(_) => {}
            Err(e) => error!("Gaming-news task error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_news(&data.db, 14).await {
            warn!("Could not prune seen_news: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}

/// Colour per news source.
fn source_colour(source: &str) -> u32 {
    match source {
        "GamesRadar"         => 0xFF6600,
        "Eurogamer"          => 0x0070CC,
        "Rock Paper Shotgun" => 0x009900,
        "PC Gamer"           => 0xCC0000,
        "VG247"              => 0x7C3AED,
        _                    => 0x5865F2,
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let relevant: Vec<_> = configs
        .into_iter()
        .filter(|c| c.news_channel_id.is_some())
        .collect();

    if relevant.is_empty() {
        return Ok(0);
    }

    let articles = fetcher::fetch_all_feeds(&data.http_client).await;
    let mut total = 0usize;

    for cfg in relevant {
        let channel_id_str = cfg.news_channel_id.as_ref().unwrap();
        let channel_id_u64: u64 = match channel_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                warn!("Invalid news channel_id for guild {}", cfg.guild_id);
                continue;
            }
        };
        let channel = serenity::ChannelId::new(channel_id_u64);

        for article in &articles {
            if queries::is_news_seen(&data.db, &cfg.guild_id, &article.id).await? {
                continue;
            }

            queries::mark_news_seen(&data.db, &cfg.guild_id, &article.id).await?;

            let mut embed = serenity::CreateEmbed::new()
                .title(&article.title)
                .url(&article.url)
                .color(source_colour(&article.source))
                .footer(serenity::CreateEmbedFooter::new(format!(
                    "🎮 {} • Gaming News",
                    article.source
                )));

            if let Some(desc) = &article.description {
                embed = embed.description(desc);
            }
            if let Some(img) = &article.image_url {
                embed = embed.thumbnail(img);
            }
            if let Some(ts) = article.published_at {
                if let Ok(timestamp) = serenity::Timestamp::from_unix_timestamp(ts.timestamp()) {
                    embed = embed.timestamp(timestamp);
                }
            }

            let msg = serenity::CreateMessage::new().embed(embed);

            match channel.send_message(http, msg).await {
                Ok(_) => {
                    info!("📰 Posted article '{}' to {}", article.title, channel_id_str);
                    total += 1;
                }
                Err(e) => {
                    error!("Failed to post news article to {}: {}", channel_id_str, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    Ok(total)
}
