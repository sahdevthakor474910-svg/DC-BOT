use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use crate::reddit::client::{RedditClient, SUBREDDITS, NSFW_SUBREDDITS, JAV_SUBREDDITS};

/// Run a single fetch-and-post cycle (used by /admin force-refresh).
pub async fn run_once(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    tick(data, http).await
}

/// Entry point for the background meme-fetching task.
/// Spawned once on bot startup; runs indefinitely.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🚀 Meme background task started");

    loop {
        match tick(&data, &http).await {
            Ok(posted) => {
                if posted > 0 {
                    info!("✅ Posted {} new meme(s) this tick", posted);
                }
            }
            Err(e) => {
                error!("❌ Meme task error: {:#}", e);
            }
        }

        // Prune old deduplication records once per tick
        if let Err(e) = queries::prune_old_seen_posts(&data.db, 30).await {
            warn!("Could not prune seen_posts: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

/// Helper to get target channel ID based on subreddit.
fn get_target_channel_id(cfg: &queries::GuildConfig, subreddit: &str) -> Option<String> {
    match subreddit.to_lowercase().as_str() {
        "memes" | "dankmemes" => cfg.meme_channel_id.clone(),
        "brainrot" => cfg.brainrot_channel_id.clone(),
        "shitposting" | "whenthe" => cfg.shitposting_channel_id.clone(),
        "196" => cfg.instagram_channel_id.clone(),
        _ => None,
    }
}

/// Fetch posts from one subreddit and post unseen ones to a Discord channel.
/// Returns the number of posts successfully sent.
async fn post_subreddit(
    data: &Data,
    http: &Arc<serenity::Http>,
    guild_id: &str,
    subreddit: &str,
    channel_id_str: &str,
) -> usize {
    let channel_id_u64: u64 = match channel_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid channel ID in DB for guild {}: {}", guild_id, channel_id_str);
            return 0;
        }
    };

    let channel = serenity::ChannelId::new(channel_id_u64);
    let mut posted = 0usize;

    match data.reddit.fetch_hot_posts(subreddit, 15).await {
        Ok(posts) => {
            for post in posts {
                // Skip already-seen posts
                match queries::is_post_seen(&data.db, guild_id, &post.id).await {
                    Ok(true) => continue,
                    Err(e) => { error!("DB error checking seen post: {}", e); continue; }
                    _ => {}
                }

                // Mark seen immediately so we don't retry un-postable content
                if let Err(e) = queries::mark_post_seen(&data.db, guild_id, &post.id).await {
                    error!("DB error marking post seen: {}", e);
                }

                // SPAM PREVENTION: Only post a maximum of 2 items per subreddit per tick.
                // We still mark the other 13 fetched items as 'seen' above, so they won't 
                // be posted later either. This safely handles database resets/redeploys.
                if posted >= 2 {
                    continue;
                }

                // Resolve embeddable media URL
                let media_url = match RedditClient::media_url(&post) {
                    Some(u) => u,
                    None => continue,
                };

                // Build Discord Embed
                let embed = serenity::CreateEmbed::new()
                    .title(&post.title)
                    .url(&post.permalink)   // already a full URL from meme-api
                    .image(&media_url)
                    .footer(serenity::CreateEmbedFooter::new(format!(
                        "r/{} • 👍 {} • by u/{}",
                        post.subreddit, post.score, post.author
                    )))
                    .color(0xFF4500); // Reddit orange-red

                let message = serenity::CreateMessage::new().embed(embed);

                if let Err(e) = channel.send_message(http, message).await {
                    error!(
                        "Failed to post {} to channel {}: {}",
                        post.id, channel_id_str, e
                    );
                } else {
                    info!("📸 Posted r/{} post {} to {}", subreddit, post.id, channel_id_str);
                    posted += 1;
                }

                // Small delay between posts to avoid rate-limiting
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        Err(e) => {
            error!("Failed to fetch r/{}: {:#}", subreddit, e);
        }
    }

    posted
}

/// One sweep: for every guild with configured channels, fetch all subreddits and
/// post any unseen media posts as embeds.
async fn tick(data: &Data, http: &Arc<serenity::Http>) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;

    if configs.is_empty() {
        return Ok(0);
    }

    let mut total_posted = 0usize;

    for cfg in configs {
        // ── Regular subreddits ────────────────────────────────────────────
        for subreddit in SUBREDDITS {
            let channel_id_str = match get_target_channel_id(&cfg, subreddit) {
                Some(id) => id,
                None => continue,
            };
            total_posted += post_subreddit(data, http, &cfg.guild_id, subreddit, &channel_id_str).await;
        }

        // ── NSFW subreddits ──────────────────────────────────────────────
        for subreddit in NSFW_SUBREDDITS {
            let target_channel = match *subreddit {
                "rule34" => cfg.rule34_channel_id.as_ref().or(cfg.nsfw_channel_id.as_ref()),
                "hentai" => cfg.hentai_channel_id.as_ref().or(cfg.nsfw_channel_id.as_ref()),
                "RealGirls" | "milf" | "boobs" | "amateur" => cfg.porn_channel_id.as_ref().or(cfg.nsfw_channel_id.as_ref()),
                _ => cfg.nsfw_channel_id.as_ref(),
            };

            if let Some(channel_id) = target_channel {
                total_posted += post_subreddit(data, http, &cfg.guild_id, subreddit, channel_id).await;
            }
        }

        // ── JAV subreddits (r/jav, r/javonline) ────────────────────────────────
        if let Some(ref jav_channel) = cfg.jav_channel_id.clone() {
            for subreddit in JAV_SUBREDDITS {
                total_posted += post_subreddit(data, http, &cfg.guild_id, subreddit, jav_channel).await;
            }
        }
    }

    Ok(total_posted)

}
