use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use crate::reddit::client::{RedditClient, JAV_SUBREDDITS};

/// Single tick exposed for `/admin force-refresh`.
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    tick(data, http).await
}

/// Background task — runs every 15 minutes.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🎌 JAV task started (r/jav + r/javonline via meme-api)");

    loop {
        match tick(&data, &http).await {
            Ok(n) if n > 0 => info!("🎌 Posted {} JAV post(s)", n),
            Ok(_) => {}
            Err(e) => error!("JAV task error: {:#}", e),
        }

        if let Err(e) = queries::prune_old_seen_jav(&data.db, 30).await {
            warn!("Could not prune seen_jav: {}", e);
        }

        // Run every 30 minutes — same cadence as free games
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}

async fn tick(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
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

        for subreddit in JAV_SUBREDDITS {
            match data.reddit.fetch_hot_posts(subreddit, 10).await {
                Ok(posts) => {
                    let mut posted_this_subreddit = 0usize;
                    for post in posts {
                        // Dedup using seen_jav table
                        match queries::is_jav_seen(&data.db, &cfg.guild_id, &post.id).await {
                            Ok(true) => continue,
                            Err(e) => { error!("DB error checking seen_jav: {}", e); continue; }
                            _ => {}
                        }

                        if let Err(e) = queries::mark_jav_seen(&data.db, &cfg.guild_id, &post.id).await {
                            error!("DB error marking jav seen: {}", e);
                        }

                        // Limit to 5 posts per subreddit per tick to keep channels active.
                        if posted_this_subreddit >= 5 {
                            continue;
                        }

                        let media_url = match RedditClient::media_url(&post) {
                            Some(u) => u,
                            None => continue,
                        };

                        let embed = serenity::CreateEmbed::new()
                            .title(&post.title)
                            .url(&post.permalink)
                            .image(&media_url)
                            .color(0xFF3366) // Hot pink for JAV
                            .footer(serenity::CreateEmbedFooter::new(format!(
                                "🎌 r/{} • 👍 {} • by u/{}",
                                post.subreddit, post.score, post.author
                            )));

                        let msg = serenity::CreateMessage::new().embed(embed);

                        match channel.send_message(http, msg).await {
                            Ok(_) => {
                                info!("🎌 Posted JAV post {} to guild {}", post.id, cfg.guild_id);
                                total += 1;
                                posted_this_subreddit += 1;
                            }
                            Err(e) => {
                                error!("Failed to post JAV to channel {}: {}", channel_id_str, e);
                            }
                        }

                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
                Err(e) => {
                    error!("Failed to fetch r/{}: {:#}", subreddit, e);
                }
            }
        }
    }

    Ok(total)
}
