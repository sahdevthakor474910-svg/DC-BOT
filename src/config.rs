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
    /// Public base URL of the bot server for rendering web-stream links.
    pub public_url: String,
    /// Google Gemini API key — used for DMC: Peak of Combat screenshot analysis.
    /// Optional: feature is silently disabled when absent.
    pub gemini_api_key: String,
}

impl AppConfig {
    /// Load config from environment, returning a descriptive error for any
    /// missing required variable.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            discord_token: env::var("DISCORD_TOKEN")
                .context("Missing required env var DISCORD_TOKEN")?
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string(),
            discord_client_id: env::var("DISCORD_CLIENT_ID")
                .context("Missing required env var DISCORD_CLIENT_ID")?
                .parse::<u64>()
                .context("DISCORD_CLIENT_ID must be a valid u64")?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/bot.db".to_string()),
            reddit_user_agent: env::var("REDDIT_USER_AGENT")
                .unwrap_or_else(|_| "discord-meme-bot/1.0".to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            public_url: {
                let render_url = env::var("RENDER_EXTERNAL_URL")
                    .or_else(|_| env::var("RENDER_EXTERNAL_HOSTNAME").map(|h| format!("https://{}", h)));
                let pub_url = env::var("PUBLIC_URL").ok();

                match (render_url, pub_url) {
                    // On Render, RENDER_EXTERNAL_URL is always accurate to the currently running service.
                    // Only use PUBLIC_URL if it's a custom domain (i.e. does not end in onrender.com)
                    (Ok(_r_url), Some(p_url)) if !p_url.contains("onrender.com") && p_url.starts_with("http") => p_url,
                    (Ok(r_url), _) => r_url,
                    // If not on Render, use PUBLIC_URL as long as it's not the old suspended domain
                    (Err(_), Some(p_url)) if !p_url.contains("oz1r") && p_url.starts_with("http") => p_url,
                    _ => env::var("RAILWAY_STATIC_URL")
                        .map(|u| format!("https://{}", u))
                        .unwrap_or_else(|_| "https://dc-bot-3-l631.onrender.com".to_string()),
                }
            }
            .trim_end_matches('/')
            .to_string(),
            gemini_api_key: env::var("GEMINI_API_KEY").unwrap_or_default(),
        })
    }
}
