#![allow(dead_code)]

mod ai;
mod commands;
mod config;
mod coc;
mod data;
mod db;
mod dmc;
mod events;
mod freegames;
mod gali;
mod hanime;
mod jav;
mod javhd;
mod news;
mod okxxx;
mod porn;
mod pornclips;
mod reddit;
mod twitter;
mod web;
mod xnxx;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::data::{Data, Error};

/// A non-blocking tracing layer that captures formatted log lines into a ring
/// buffer. Unlike the old `MemoryLogWriter` this does NOT touch stdout (the
/// default `fmt::Layer` handles that) and never blocks the async runtime.
struct RingBufferLayer(Arc<Mutex<VecDeque<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for RingBufferLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use std::fmt::Write;
        let meta = event.metadata();
        let mut msg = String::new();
        let _ = write!(msg, "{} ", meta.level());

        // Visitor to extract the "message" field
        struct MsgVisitor<'a>(&'a mut String);
        impl tracing::field::Visit for MsgVisitor<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    let _ = write!(self.0, "{:?}", value);
                } else {
                    let _ = write!(self.0, " {}={:?}", field.name(), value);
                }
            }
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "message" {
                    self.0.push_str(value);
                } else {
                    let _ = write!(self.0, " {}={}", field.name(), value);
                }
            }
        }
        event.record(&mut MsgVisitor(&mut msg));
        msg.push('\n');

        if let Ok(mut lock) = self.0.lock() {
            if lock.len() >= 500 {
                lock.pop_front();
            }
            lock.push_back(msg);
        }
    }
}

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
            if let Ok(mut lock) = bot_data.status.write() {
                *lock = format!("Online as {} ({})", data_about_bot.user.name, data_about_bot.user.id);
            }
        }

        serenity::FullEvent::Message { new_message } => {
            if let Err(e) = events::message::handle(ctx, new_message, bot_data).await {
                error!("Message event error: {:#}", e);
            }
        }

        serenity::FullEvent::InteractionCreate { interaction } => {
            info!("⚡ InteractionCreate event: kind={:?}, id={}", interaction.kind(), interaction.id());
        }

        serenity::FullEvent::GuildCreate { guild, is_new } => {
            if is_new.unwrap_or(false) {
                info!("🎉 Joined new guild: {} ({}) - registering commands instantly", guild.name, guild.id);
                if let Err(e) = poise::builtins::register_in_guild(ctx, &_framework.options().commands, guild.id).await {
                    tracing::warn!("Failed to register commands in new guild {}: {:?}", guild.id, e);
                } else {
                    info!("⚡ Registered commands instantly in new guild {}", guild.id);
                }
            }
        }

        serenity::FullEvent::Ratelimit { data } => {
            warn!(
                "⚠️ Discord rate-limited! timeout: {:?}, limit: {}, method: {:?}, path: {:?}, global: {}",
                data.timeout, data.limit, data.method, data.path, data.global
            );
            if let Ok(mut lock) = bot_data.status.write() {
                *lock = format!("Rate-limited: timeout={:?}, path={:?}", data.timeout, data.path);
            }
        }

        serenity::FullEvent::Resume { .. } => {
            info!("🔄 Gateway session resumed");
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

    let log_buffer = Arc::new(Mutex::new(VecDeque::<String>::new()));

    // Initialise structured logging with two layers:
    // 1. fmt::Layer → writes to stdout (default, non-blocking)
    // 2. RingBufferLayer → captures lines into an in-memory ring buffer for /logs
    let default_filter = format!("{},serenity::http=debug,serenity::client=debug", app_config.log_level);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&default_filter));
    let fmt_layer = fmt::layer().with_target(false);
    let ring_layer = RingBufferLayer(log_buffer.clone());
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(ring_layer)
        .init();

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

    // ── Start web server FIRST so Render health check passes immediately ─────
    // Render scans for an open port right after process start. If we wait until
    // the Discord handshake completes (can take minutes if rate-limited), Render times out
    // and kills the container. We spawn the web server here with endpoints that return
    // 200 OK immediately, keeping Render happy regardless of Discord gateway status.
    {
        let port = std::env::var("PORT").unwrap_or_else(|_| "10000".to_string());
        let addr = format!("0.0.0.0:{}", port);
        info!("📡 Starting web server on {} (before Discord connect)…", addr);
        let logs_clone = log_buffer.clone();
        let status_clone = bot_data.status.clone();
        tokio::spawn(async move {
            use axum::{routing::get, Router};
            let app = Router::new()
                .route("/", get(|| async { "OK" }))
                .route("/health", get(|| async { "OK" }))
                .route("/status", get(move || {
                    let status = status_clone.clone();
                    async move {
                        status
                            .read()
                            .map(|s| s.clone())
                            .unwrap_or_else(|_| "Unknown".to_string())
                    }
                }))
                .route("/logs", get(move || {
                    let logs = logs_clone.clone();
                    async move {
                        let lock = logs.lock().unwrap();
                        lock.iter().cloned().collect::<Vec<_>>().join("")
                    }
                }));
            match tokio::net::TcpListener::bind(&addr).await {
                Ok(listener) => {
                    info!("📡 Web server listening on http://{}", addr);
                    if let Err(e) = axum::serve(listener, app).await {
                        error!("❌ Web server failed: {}", e);
                    }
                }
                Err(e) => error!("❌ Web server failed to bind to {}: {}", addr, e),
            }
        });
    }

    // Gateway intents
    // MESSAGE_CONTENT is privileged — must be enabled in the Developer Portal.
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    // Start Serenity client with automatic reconnect loop
    let token = app_config.discord_token.clone();
    info!(
        "🔧 Discord config: client_id: {}, token_len: {}, token_prefix: {}…",
        app_config.discord_client_id,
        token.len(),
        &token[..token.len().min(10)]
    );

    let mut attempt = 1u64;
    loop {
        info!("🚀 Connecting to Discord Gateway (attempt {})...", attempt);
        if let Ok(mut lock) = bot_data.status.write() {
            *lock = format!("Connecting to Discord Gateway (attempt {})...", attempt);
        }

        let framework = build_framework(bot_data.clone());
        let builder = serenity::ClientBuilder::new(&token, intents).framework(framework);

        // Allow up to 15 minutes (900s) for Serenity to wait out any global 429 rate limit.
        // We NEVER exit the process with exit(1) on failure or timeout.
        // Exiting causes Render to restart the container, which immediately sends another
        // request to Discord and resets/extends the rate limit block.
        // The web server on port 10000 stays alive so Render considers the service healthy throughout.
        match tokio::time::timeout(std::time::Duration::from_secs(900), builder).await {
            Ok(Ok(mut client)) => {
                info!("✅ Discord client built successfully! Starting gateway connection…");
                attempt = 1; // reset on successful build

                match client.start().await {
                    Ok(()) => {
                        info!("Discord client session exited cleanly. Reconnecting in 10s…");
                        if let Ok(mut lock) = bot_data.status.write() {
                            *lock = "Disconnected cleanly. Reconnecting in 10s…".to_string();
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                    Err(e) => {
                        let err_str = format!("{:#}", e);
                        error!("❌ Discord client error / disconnected: {}. Reconnecting in 30s…", err_str);
                        if let Ok(mut lock) = bot_data.status.write() {
                            *lock = format!("Disconnected: {}. Reconnecting in 30s…", err_str);
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    }
                }
            }
            Ok(Err(e)) => {
                let err_str = format!("{:#}", e);
                error!("❌ Failed to build Discord client: {}. Waiting 60s before retry…", err_str);
                if let Ok(mut lock) = bot_data.status.write() {
                    *lock = format!("Failed to build client: {}. Retrying in 60s…", err_str);
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                attempt += 1;
            }
            Err(_) => {
                warn!("⚠️ Discord client builder timed out after 15 minutes. Retrying in 30s…");
                if let Ok(mut lock) = bot_data.status.write() {
                    *lock = "Client builder timed out (15m). Retrying in 30s…".to_string();
                }
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                attempt += 1;
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Framework builder
// ────────────────────────────────────────────────────────────────────────────

fn build_framework(bot_data: Data) -> poise::Framework<Data, Error> {
    poise::Framework::builder()
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
            // ── Pre-command hook: auto-defer all slash commands immediately ──
            pre_command: |ctx| {
                Box::pin(async move {
                    info!("▶️ Slash command /{} invoked by {} ({})", ctx.command().name, ctx.author().name, ctx.author().id);
                    if let poise::Context::Application(_) = ctx {
                        let _ = ctx.defer().await;
                    }
                })
            },
            // ── Global check: silently block banned users on every command ──
            command_check: Some(|ctx| {
                Box::pin(commands::checks::is_not_blocked_check(ctx))
            }),
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            let bot_data = bot_data.clone();
            let http     = Arc::clone(&ctx.http);

            Box::pin(async move {
                // Collect all guild IDs from Gateway Ready event, Cache, and DB configs
                let mut guild_ids = std::collections::HashSet::new();

                for unavailable_guild in &ready.guilds {
                    guild_ids.insert(unavailable_guild.id);
                }

                for guild_id in ctx.cache.guilds() {
                    guild_ids.insert(guild_id);
                }

                if let Ok(configs) = crate::db::queries::get_all_guild_configs(&bot_data.db).await {
                    for cfg in configs {
                        if let Ok(guild_id_num) = cfg.guild_id.parse::<u64>() {
                            guild_ids.insert(serenity::GuildId::new(guild_id_num));
                        }
                    }
                }

                // Register slash commands in each guild (instant 0s availability)
                // Small delay between guilds to avoid Discord 429 rate limits
                info!("⚡ Registering slash commands across {} guild(s)...", guild_ids.len());
                for guild_id in &guild_ids {
                    if let Err(e) = poise::builtins::register_in_guild(ctx, &framework.options().commands, *guild_id).await {
                        tracing::warn!("Could not register commands in guild {}: {:?}", guild_id, e);
                    } else {
                        info!("⚡ Slash commands registered in guild {}", guild_id);
                    }
                    // Small delay to avoid hitting Discord rate limits
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                info!("✅ Slash commands registered in all {} guild(s)", guild_ids.len());

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

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { twitter::task::run(d, h).await });
                }
                info!("⏱️  Twitter/X task spawned (every 10 min — @dmc_poc & @dmc_poc_jp via Nitter RSS)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { xnxx::task::run(d, h).await });
                }
                info!("⏱️  XNXX task spawned (every 30 min — xnxx.com trending)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { javhd::task::run(d, h).await });
                }
                info!("⏱️  JAVHD task spawned (every 20 min — javhd.com API)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { hanime::task::run(d, h).await });
                }
                info!("⏱️  Hentai task spawned (every 20 min — anime & 3D hentai with direct MP4)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { pornclips::task::run(d, h).await });
                }
                info!("⏱️  Porn Clips task spawned (every 10 min — RedGIFs short clips)");

                Ok(bot_data)
            })
        })
        .build()
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

    #[tokio::test]
    #[ignore = "Scrolller API blocks anonymous sandbox requests; works in production"]
    async fn test_scrolller_fetch() {
        let reddit_client = reddit::client::RedditClient::new("discord-meme-bot/1.0 (by /u/SahdevXD)").unwrap();
        let posts = reddit_client.fetch_hot_posts("nsfw", 3).await;
        assert!(posts.is_ok(), "Failed to fetch NSFW posts from Scrolller: {:?}", posts.err());
        let posts = posts.unwrap();
        println!("Fetched NSFW posts from Scrolller: {:?}", posts);
        assert!(!posts.is_empty(), "Scrolller posts list should not be empty");
    }

    #[tokio::test]
    async fn test_japanese_translation() {
        let client = reqwest::Client::new();
        
        let jp_text = "こんにちは、世界！";
        assert!(twitter::translate::is_japanese(jp_text), "Should detect Japanese text");

        let en_text = "Hello, world!";
        assert!(!twitter::translate::is_japanese(en_text), "Should detect SFW/non-Japanese text as false");

        let translated = twitter::translate::translate_ja_to_en(&client, jp_text, "").await;
        println!("Translated '{}' -> '{}'", jp_text, translated);
        assert!(!translated.is_empty());
    }

    #[tokio::test]
    async fn test_live_translation() {
        let twitter_client = twitter::client::TwitterClient::new().unwrap();
        println!("Fetching tweets from @dmc_poc_jp...");
        let tweets = twitter_client.fetch_tweets("dmc_poc_jp", 5).await;
        assert!(tweets.is_ok(), "Failed to fetch tweets: {:?}", tweets.err());
        let tweets = tweets.unwrap();
        for (i, tweet) in tweets.iter().enumerate() {
            println!("\n--- Tweet {} ---", i + 1);
            println!("  ID: {}", tweet.id);
            println!("  Text: {}", tweet.text);
            let is_jp = twitter::translate::is_japanese(&tweet.text);
            println!("  Is Japanese? {}", is_jp);
            if is_jp {
                let translated = twitter::translate::translate_ja_to_en(twitter_client.http(), &tweet.text, "").await;
                println!("  Translated: {}", translated);
            }
        }
    }

    #[test]
    fn test_dmc_calculator() {
        use crate::dmc::calculator::build_discord_message;
        use crate::dmc::gemini::{LeaderboardPlayer, ScreenshotData};

        // Test case 1: Results screen – Hell Commander with X120% bonus.
        // reward_pts is read directly from the screenshot (small number ~1.4M),
        // NOT computed from boss_pts - dmg_pts (which would give ~0 for big bosses).
        //   reward_pts = 1_370_684  →  secs_remaining ≈ 28.0s  →  kill_time ≈ 272s
        let results = ScreenshotData::Results {
            boss_name: "Hell Commander".to_string(),
            dmg_pts: 2_892_440_140,
            reward_pts: 1_370_684, // directly read from "Reward PTS" row
            boss_pts: 2_893_810_824,
            has_bonus: true,
        };
        let msg = build_discord_message(&results);
        assert!(msg.contains("Hell Commander Results"));
        assert!(msg.contains("X120% ✓"));
        assert!(!msg.contains("Kill Time   : 0s"), "Kill time should not be zero");
        assert!(!msg.contains("Kill Time   : 5m 0"), "Kill time should not be 5 min exactly (Dante bug)");
        println!("Results output:\n{}", msg);

        // Test case 1b: Dante post-update with X120% bonus (7_231_072_000 DMG PTS).
        let dante_results = ScreenshotData::Results {
            boss_name: "Dante".to_string(),
            dmg_pts: 7_231_072_000,
            reward_pts: 11_990_058,
            boss_pts: 8_691_674_470,
            has_bonus: true,
        };
        let dante_msg = build_discord_message(&dante_results);
        assert!(dante_msg.contains("Dante Results"));
        assert!(dante_msg.contains("Kill Time   : 55.1s"), "Dante kill time should be 55.1s");
        println!("Dante Results output:\n{}", dante_msg);

        // Test case 2: Leaderboard screen – Calibur without bonus
        let leaderboard = ScreenshotData::Leaderboard {
            boss_name: "Calibur".to_string(),
            has_bonus: false,
            players: vec![
                LeaderboardPlayer { rank: 1, name: "中國台灣省".to_string(), total_pts: 1_033_499_653 },
                LeaderboardPlayer { rank: 2, name: "KèLiêuMạng.VN".to_string(), total_pts: 1_033_179_794 },
                LeaderboardPlayer { rank: 3, name: "Desuwyy!".to_string(), total_pts: 1_032_576_203 },
                LeaderboardPlayer { rank: 4, name: "★PinjamDulu`Seratus★".to_string(), total_pts: 1_030_632_084 },
            ],
        };
        let msg2 = build_discord_message(&leaderboard);
        assert!(msg2.contains("Calibur Leaderboard"));
        assert!(msg2.contains("🥇"));
        assert!(msg2.contains("中國台灣省"));
        assert!(msg2.contains("Kill Time"));
        println!("Leaderboard output:\n{}", msg2);

        // Test case 3: Leaderboard – Hell Shade with X120% bonus
        let lb_bonus = ScreenshotData::Leaderboard {
            boss_name: "Hell Shade".to_string(),
            has_bonus: true,
            players: vec![
                LeaderboardPlayer { rank: 1, name: "S-OH-Am".to_string(), total_pts: 1_099_993_341 },
                LeaderboardPlayer { rank: 2, name: "Tanishq".to_string(), total_pts: 1_097_568_228 },
                LeaderboardPlayer { rank: 3, name: "IncompletePlayer".to_string(), total_pts: 500_000_000 },
            ],
        };
        let msg3 = build_discord_message(&lb_bonus);
        assert!(msg3.contains("Hell Shade Leaderboard"));
        assert!(msg3.contains("X120%"));
        assert!(msg3.contains("Boss Not Killed"), "Leaderboard incomplete player should show Boss Not Killed");
        println!("Hell Shade Leaderboard output:\n{}", msg3);

        // Test case 4: Results screen with Gemini row-swap (boss_pts < dmg_pts)
        let swapped = ScreenshotData::Results {
            boss_name: "Vergil".to_string(),
            dmg_pts: 2_893_810_824, // larger (actually boss_pts)
            reward_pts: 0,
            boss_pts: 2_892_440_140, // smaller (actually dmg_pts)
            has_bonus: true,
        };
        let msg4 = build_discord_message(&swapped);
        assert!(msg4.contains("Vergil Results"));
        assert!(!msg4.contains("5m 0.0s"), "Swapped fields should be auto-swapped");
        println!("Swapped fields output:\n{}", msg4);

        // Test case 5: Results screen with hallucinated large reward_pts (> 20M)
        let hallucinated = ScreenshotData::Results {
            boss_name: "Beowulf".to_string(),
            dmg_pts: 946_374_652,
            reward_pts: 946_374_652, // hallucinated boss_pts into reward_pts
            boss_pts: 950_000_000,
            has_bonus: true,
        };
        let msg5 = build_discord_message(&hallucinated);
        assert!(msg5.contains("Beowulf Results"));
        println!("Hallucinated reward_pts output:\n{}", msg5);

        // Test case 6: Truncated 10-digit score on Dante Leaderboard (e.g. BADJASS missing trailing digit)
        let dante_lb = ScreenshotData::Leaderboard {
            boss_name: "Dante".to_string(),
            has_bonus: true,
            players: vec![
                LeaderboardPlayer { rank: 1, name: "BADJASS".to_string(), total_pts: 869_198_432 }, // truncated 9-digit
                LeaderboardPlayer { rank: 2, name: "ATHiINA".to_string(), total_pts: 8_690_908_101 },
            ],
        };
        let msg6 = build_discord_message(&dante_lb);
        assert!(msg6.contains("Dante Leaderboard"));
        assert!(msg6.contains("BADJASS"), "Leaderboard should include BADJASS");
        assert!(!msg6.contains("BADJASS\n    Total PTS : 869198432\n    Kill Time : ❌ Boss Not Killed"), "BADJASS truncated score should be auto-corrected");
        println!("Truncated Dante Leaderboard output:\n{}", msg6);

        // Test case 7: Minotaur Results & Leaderboard (exact values from screenshots)
        let mino_results = ScreenshotData::Results {
            boss_name: "Minotaur".to_string(),
            dmg_pts: 5_783_842_000,
            reward_pts: 4_140_934,
            boss_pts: 6_945_579_521,
            has_bonus: true,
        };
        let msg7a = build_discord_message(&mino_results);
        assert!(msg7a.contains("Minotaur Results"));
        assert!(msg7a.contains("2m 35.4s"), "Minotaur kill time should be 2m 35.4s");
        println!("Minotaur Results output:\n{}", msg7a);

        let mino_lb = ScreenshotData::Leaderboard {
            boss_name: "Minotaur".to_string(),
            has_bonus: true,
            players: vec![
                LeaderboardPlayer { rank: 1, name: "Rivo2".to_string(), total_pts: 6_951_437_021 },
                LeaderboardPlayer { rank: 2, name: "Grymm_Jaergliff".to_string(), total_pts: 6_945_579_521 },
                LeaderboardPlayer { rank: 3, name: "МёртвыйАнархист".to_string(), total_pts: 6_945_414_746 },
                LeaderboardPlayer { rank: 4, name: "Theskylin".to_string(), total_pts: 6_943_349_614 },
            ],
        };
        let msg7b = build_discord_message(&mino_lb);
        assert!(msg7b.contains("Minotaur Leaderboard"));
        assert!(msg7b.contains("Rivo2"));
        assert!(msg7b.contains("55.7s"), "Rivo2 kill time should be 55.7s");
        assert!(msg7b.contains("2m 35.4s"), "Grymm_Jaergliff kill time should be 2m 35.4s");
        assert!(!msg7b.contains("Boss Not Killed"), "Minotaur full clear should have valid kill time");
        println!("Minotaur Leaderboard output:\n{}", msg7b);
    }
}


