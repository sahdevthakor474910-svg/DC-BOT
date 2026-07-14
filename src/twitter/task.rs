use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::data::Data;
use crate::db::queries;
use super::client::{TwitterClient, ACCOUNTS};

/// Background task — polls every 10 minutes for new tweets from both accounts.
pub async fn run(data: Data, http: Arc<serenity::Http>) {
    info!("🐦 Twitter/X task started — monitoring @dmc_poc & @dmc_poc_jp via Nitter RSS");

    let client = match TwitterClient::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create TwitterClient: {:#}", e);
            return;
        }
    };

    loop {
        match tick(&data, &http, &client).await {
            Ok(n) if n > 0 => info!("🐦 Twitter: posted {} new tweet(s)", n),
            Ok(_) => {}
            Err(e) => error!("Twitter task error: {:#}", e),
        }

        tokio::time::sleep(Duration::from_secs(10 * 60)).await;
    }
}

async fn tick(
    data: &Data,
    http: &Arc<serenity::Http>,
    client: &TwitterClient,
) -> Result<usize> {
    let configs = queries::get_all_guild_configs(&data.db).await?;
    let mut total = 0usize;

    for (username, label) in ACCOUNTS {
        // Fetch up to 10 latest tweets per account
        let tweets = match client.fetch_tweets(username, 10).await {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to fetch tweets for @{}: {}", username, e);
                continue;
            }
        };

        for cfg in &configs {
            // Determine target channel for this username
            let target_channel_id = if *username == "dmc_poc" {
                cfg.twitter_global_channel_id.as_ref().or(cfg.twitter_channel_id.as_ref())
            } else if *username == "dmc_poc_jp" {
                cfg.twitter_asia_channel_id.as_ref().or(cfg.twitter_channel_id.as_ref())
            } else {
                None
            };

            let channel_id_str = match target_channel_id {
                Some(id) => id,
                None => continue,
            };

            let channel_id_u64: u64 = match channel_id_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    warn!("Invalid channel id {} for guild {}", channel_id_str, cfg.guild_id);
                    continue;
                }
            };
            let channel = serenity::ChannelId::new(channel_id_u64);

            for tweet in &tweets {
                // Deduplication — skip if already posted
                match queries::is_tweet_seen(&data.db, &cfg.guild_id, &tweet.id).await {
                    Ok(true) => continue,
                    Err(e) => {
                        error!("DB error checking seen_tweets: {}", e);
                        continue;
                    }
                    _ => {}
                }

                if let Err(e) = queries::mark_tweet_seen(&data.db, &cfg.guild_id, &tweet.id).await {
                    error!("DB error marking tweet seen: {}", e);
                }

                // Build a clean embed
                let description = if tweet.text.len() > 1800 {
                    format!("{}…", &tweet.text[..1800])
                } else {
                    tweet.text.clone()
                };

                let footer_text = if tweet.pub_date.is_empty() {
                    format!("𝕏 @{}", tweet.account)
                } else {
                    format!("𝕏 @{} • {}", tweet.account, tweet.pub_date)
                };

                let embed = serenity::CreateEmbed::new()
                    .author(
                        serenity::CreateEmbedAuthor::new(format!("{} (@{})", label, username))
                            .url(format!("https://twitter.com/{}", username))
                            .icon_url("https://abs.twimg.com/favicons/twitter.3.ico"),
                    )
                    .description(&description)
                    .url(&tweet.link)
                    .color(0x1DA1F2) // Classic Twitter blue
                    .footer(serenity::CreateEmbedFooter::new(footer_text));

                let msg = serenity::CreateMessage::new()
                    .content(format!("📣 New update from **{}** ([@{}]({})):", label, username, tweet.link))
                    .embed(embed);

                match channel.send_message(http, msg).await {
                    Ok(_) => {
                        info!("🐦 Posted tweet {} (@{}) to guild {}", tweet.id, username, cfg.guild_id);
                        total += 1;
                    }
                    Err(e) => {
                        error!("Failed to post tweet to channel {}: {}", channel_id_str, e);
                    }
                }

                // Slight delay between posts to respect Discord rate limits
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
        }

        // Brief pause between the two accounts
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    Ok(total)
}
