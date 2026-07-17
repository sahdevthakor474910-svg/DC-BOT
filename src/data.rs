use std::sync::Arc;

use anyhow::Result;
use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::reddit::client::RedditClient;

// ────────────────────────────────────────────────────────────────────────────
// Type aliases used throughout the bot
// ────────────────────────────────────────────────────────────────────────────

/// The bot's error type — a boxed anyhow error for maximum flexibility.
pub type Error = anyhow::Error;

/// Poise command context parameterised with our shared data and error types.
pub type Context<'a> = poise::Context<'a, Data, Error>;

// ────────────────────────────────────────────────────────────────────────────
// Shared bot state (passed to every command and event handler)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Data {
    /// SQLite connection pool.
    pub db: SqlitePool,
    /// Shared Reddit API client.
    pub reddit: Arc<RedditClient>,
    /// Shared general reqwest client for RSS feeds and other APIs.
    pub http_client: reqwest::Client,
    /// Static application config (env-vars).
    pub config: Arc<AppConfig>,
}

impl Data {
    pub fn new(db: SqlitePool, config: AppConfig) -> Result<Self> {
        let reddit = RedditClient::new(&config.reddit_user_agent)?;
        let http_client = reqwest::Client::builder()
            .user_agent(&config.reddit_user_agent)
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        Ok(Self {
            db,
            reddit: Arc::new(reddit),
            http_client,
            config: Arc::new(config),
        })
    }
}
