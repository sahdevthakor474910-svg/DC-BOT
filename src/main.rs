#![allow(dead_code)]

mod commands;
mod config;
mod data;
mod db;
mod events;
mod freegames;
mod jav;
mod news;
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
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let bot_data = bot_data.clone();
            let http     = Arc::clone(&ctx.http);

            Box::pin(async move {
                // Register slash commands globally
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("📋 Slash commands registered globally");

                // ── Spawn background tasks ──────────────────────────────
                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { reddit::task::run(d, h).await });
                }
                info!("⏱️  Reddit meme task spawned (every 5 min)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { news::task::run(d, h).await });
                }
                info!("⏱️  Gaming-news task spawned (every 15 min)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { freegames::task::run(d, h).await });
                }
                info!("⏱️  Free-games task spawned (every 30 min)");

                {
                    let d = bot_data.clone();
                    let h = Arc::clone(&http);
                    tokio::spawn(async move { jav::task::run(d, h).await });
                }
                info!("⏱️  JAV task spawned (every 2 hours)");

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
