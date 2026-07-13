#![allow(dead_code)]

mod commands;
mod config;
mod coc;
mod data;
mod db;
mod events;
mod freegames;
mod jav;
mod news;
mod okxxx;
mod porn;
mod reddit;

use std::sync::Arc;

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use crate::data::{Data, Error};

// ────────────────────────────────────────────────────────────────────────────
// Event handler
// ────────────────────────────────────────────────────────────────────────────

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    bot_data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            info!(
                "✅ Logged in as {} ({})",
                data_about_bot.user.name,
                data_about_bot.user.id
            );
        }

        serenity::FullEvent::Message { new_message } => {
            if let Err(e) = events::message::handle(ctx, new_message, bot_data).await {
                error!("Message event error: {:#}", e);
            }
        }

        _ => {}
    }

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// Main
// ────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env (silently ignored if absent in production)
    let _ = dotenvy::dotenv();

    let app_config = config::AppConfig::from_env()
        .context("Failed to load configuration from environment")?;

    // Initialise structured logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&app_config.log_level));
    fmt().with_env_filter(filter).with_target(false).init();

    info!("🤖 Starting Discord bot v2…");

    // Ensure the data directory exists for SQLite
    if let Some(path) = app_config.database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }
    }

    // Open SQLite connection pool
    let db_options = app_config
        .database_url
        .parse::<SqliteConnectOptions>()
        .context("Invalid DATABASE_URL")?
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(db_options)
        .await
        .context("Failed to connect to SQLite")?;

    // Run all migrations (001 + 002)
    db::schema::run_migrations(&db).await?;

    // Build shared bot data
    let bot_data = Data::new(db, app_config.clone())
        .context("Failed to initialise bot data")?;

    // Gateway intents
    // MESSAGE_CONTENT is privileged — must be enabled in the Developer Portal.
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    // Build the Poise framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |error| {
                Box::pin(async move {
                    match error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            error!("Command '{}' failed: {:#}", ctx.command().name, error);
                            let _ = ctx
                                .say(format!("❌ Error: {}", error))
                                .await;
                        }
                        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                            if let Some(err) = error {
                                error!("Check for command '{}' failed with error: {:#}", ctx.command().name, err);
                                let _ = ctx.say(format!("❌ Error running permission check: {}", err)).await;
                            } else {
                                let _ = ctx
                                    .say("❌ You need the **Manage Server** permission to run this command.")
                                    .await;
                            }
                        }
                        poise::FrameworkError::Setup { error, .. } => {
                            error!("Setup error: {:#}", error);
                        }
                        other => {
                            if let Err(e) = poise::builtins::on_error(other).await {
                                error!("Unhandled framework error: {:#}", e);
                            }
                        }
                    }
                })
            },
            // ── Global check: silently block banned users on every command ──
            command_check: Some(|ctx| {
                Box::pin(commands::checks::is_not_blocked_check(ctx))
            }),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let bot_data = bot_data.clone();
            let http     = Arc::clone(&ctx.http);

            Box::pin(async move {
                // Register slash commands globally (single source of truth — no guild duplicates)
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("📋 Slash commands registered globally");

                // Clean up any stale guild-level commands that cause duplicates in Discord
                if let Ok(configs) = crate::db::queries::get_all_guild_configs(&bot_data.db).await {
                    for cfg in configs {
                        if let Ok(guild_id_num) = cfg.guild_id.parse::<u64>() {
                            let guild_id = serenity::GuildId::new(guild_id_num);
                            // Overwrite guild commands with empty list to clear any old duplicates
                            if let Err(e) = guild_id.set_commands(&ctx.http, vec![]).await {
                                tracing::warn!("Could not clear guild commands for {}: {:?}", cfg.guild_id, e);
                            } else {
                                info!("🧹 Cleared stale guild commands for guild {}", cfg.guild_id);
                            }
                        }
                    }
                }

                // ── Spawn background tasks ──────────────────────────────
                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { reddit::task::run(d, h).await });
                }
                info!("⏱️  Reddit meme task spawned (interval from DB, default 60s)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { news::task::run(d, h).await });
                }
                info!("⏱️  Gaming-news task spawned (every 5 min)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { freegames::task::run(d, h).await });
                }
                info!("⏱️  Free-games task spawned (every 15 min)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { jav::task::run(d, h).await });
                }
                info!("⏱️  JAV task spawned (every 15 min)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { porn::task::run(d, h).await });
                }
                info!("⏱️  Porn video task spawned (every 20 min — RedTube API)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { okxxx::task::run(d, h).await });
                }
                info!("⏱️  OK.XXX task spawned (every 25 min — ok.xxx scraper)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { coc::task::run(d, h).await });
                }
                info!("⏱️  CoC update task spawned (every 10 min — r/ClashOfClans + YouTube)");

                // ── Web Server for Render Health Check ───────────────────
                tokio::spawn(async move {
                    let app = axum::Router::new().route("/", axum::routing::get(|| async { "Bot is active!" }));
                    let port = std::env::var("PORT").unwrap_or_else(|_| "10000".to_string());
                    let addr = format!("0.0.0.0:{}", port);
                    info!("📡 Attempting to start web server on {}...", addr);
                    
                    match tokio::net::TcpListener::bind(&addr).await {
                        Ok(listener) => {
                            info!("📡 Web server listening on http://{}", addr);
                            if let Err(e) = axum::serve(listener, app).await {
                                error!("❌ Web server failed to serve: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("❌ Web server failed to bind to {}: {}", addr, e);
                        }
                    }
                });

                Ok(bot_data)
            })
        })
        .build();

    // Start Serenity client
    let mut client = serenity::ClientBuilder::new(&app_config.discord_token, intents)
        .framework(framework)
        .await
        .context("Failed to build Discord client")?;

    info!("🚀 Connecting to Discord…");
    client.start().await.context("Discord client exited")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "eporner.com blocks sandbox/CI IPs via Cloudflare; works from real servers"]
    async fn test_jav_eporner_search() {
        // JAV now uses eporner.com — searches for "japanese uncensored" via free Webmasters API
        let client = jav::client::EpornerClient::new().unwrap();
        let results = client.search("japanese uncensored", 3).await;
        assert!(results.is_ok(), "Failed to search eporner: {:?}", results.err());
        let results = results.unwrap();
        println!("eporner search results: {:?}", results.iter().map(|v| &v.title).collect::<Vec<_>>());
        assert!(!results.is_empty(), "eporner search should return results");
    }

    #[tokio::test]
    async fn test_reddit_meme_client_fetch() {
        let reddit_client = reddit::client::RedditClient::new("discord-meme-bot/1.0 (by /u/SahdevXD)").unwrap();
        let posts = reddit_client.fetch_hot_posts("memes", 3).await;
        assert!(posts.is_ok(), "Failed to fetch memes from meme-api: {:?}", posts.err());
        let posts = posts.unwrap();
        println!("Fetched memes: {:?}", posts);
        assert!(!posts.is_empty(), "Memes list should not be empty");
    }

    #[tokio::test]
    async fn test_news_feeds_fetch() {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .build()
            .unwrap();
        let articles = news::fetcher::fetch_feed(&client, "https://www.pcgamer.com/rss/", "PC Gamer").await;
        assert!(articles.is_ok(), "Failed to fetch PC Gamer RSS feed: {:?}", articles.err());
        let articles = articles.unwrap();
        println!("Fetched news articles: {:?}", articles);
        assert!(!articles.is_empty(), "News articles should not be empty");
    }

    #[tokio::test]
    async fn test_redtube_fetch() {
        let client = porn::client::PornClient::new().unwrap();
        let videos = client.fetch_videos("naughty america", 3).await;
        assert!(videos.is_ok(), "Failed to fetch RedTube videos: {:?}", videos.err());
        let videos = videos.unwrap();
        println!("Fetched RedTube videos: {:?}", videos);
        assert!(!videos.is_empty(), "RedTube videos list should not be empty");
    }

    #[tokio::test]
    async fn test_okxxx_fetch() {
        let client = okxxx::client::OkXxxClient::new().unwrap();
        let videos = client.fetch_videos(1).await;
        assert!(videos.is_ok(), "Failed to fetch OK.XXX videos: {:?}", videos.err());
        let videos = videos.unwrap();
        println!("Fetched OK.XXX videos: {:?}", videos);
        assert!(!videos.is_empty(), "OK.XXX videos list should not be empty");
    }

    #[tokio::test]
    async fn test_guild_fetching() {
        let _ = dotenvy::dotenv();
        let token = std::env::var("DISCORD_TOKEN").expect("token missing");
        let http = serenity::Http::new(&token);
        
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/bot.db".to_string());
        use sqlx::sqlite::SqlitePool;
        let pool = SqlitePool::connect(&db_url).await.unwrap();
        
        let rows = sqlx::query("SELECT guild_id FROM guild_config")
            .fetch_all(&pool)
            .await
            .unwrap();
            
        use sqlx::Row;
        for row in rows {
            let guild_id_str: String = row.get("guild_id");
            let guild_id: u64 = guild_id_str.parse().unwrap();
            let guild = serenity::GuildId::new(guild_id);
            println!("Testing guild: {}", guild_id);
            match guild.to_partial_guild(&http).await {
                Ok(partial) => {
                    println!("  Guild Name: {}, Owner ID: {}", partial.name, partial.owner_id);
                }
                Err(e) => {
                    println!("  Failed to fetch guild details: {:?}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_gamerpower_fetch() {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .build()
            .unwrap();
        let games = freegames::gamerpower::fetch_free_games(&client).await;
        println!("Fetched GamerPower games count: {}", games.len());
        for g in &games {
            println!("  - {} from {}", g.title, g.store);
        }
        assert!(!games.is_empty(), "GamerPower games list should not be empty");
    }
}


