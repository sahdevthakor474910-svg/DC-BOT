use anyhow::{Context as _, Result};
use std::env;

/// Top-level application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Discord bot token.
    pub discord_token: String,
    /// Discord application / client ID.
    pub discord_client_id: u64,
    /// SQLite connection string.
    pub database_url: String,
    /// Reddit API User-Agent header value.
    pub reddit_user_agent: String,
    /// Tracing log level filter (e.g. "info", "debug").
    pub log_level: String,
}

impl AppConfig {
    /// Load config from environment, returning a descriptive error for any
    /// missing required variable.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            discord_token: env::var("DISCORD_TOKEN")
                .context("Missing required env var DISCORD_TOKEN")?,
            discord_client_id: env::var("DISCORD_CLIENT_ID")
                .context("Missing required env var DISCORD_CLIENT_ID")?
                .parse::<u64>()
                .context("DISCORD_CLIENT_ID must be a valid u64")?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/bot.db".to_string()),
            reddit_user_agent: env::var("REDDIT_USER_AGENT")
                .unwrap_or_else(|_| "discord-meme-bot/1.0".to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }
}
