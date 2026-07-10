use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

use super::models::{RedditPost, RedditResponse};

/// Subreddits to poll for memes.
pub const SUBREDDITS: &[&str] = &["memes", "dankmemes", "shitposting", "brainrot", "196", "whenthe"];

// ─── OAuth2 token response ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

// ────────────────────────────────────────────────────────────────────────────

pub struct RedditClient {
    client: Client,
    client_id: String,
    client_secret: String,
    user_agent: String,
    token: tokio::sync::Mutex<Option<String>>,
}

impl RedditClient {
    pub fn new(user_agent: &str) -> Result<Self> {
        // Reddit's public read-only OAuth app credentials (script app type)
        // Falls back to anonymous-but-spoofed headers if env vars not set
        let client_id     = std::env::var("REDDIT_CLIENT_ID").unwrap_or_default();
        let client_secret = std::env::var("REDDIT_CLIENT_SECRET").unwrap_or_default();

        let client = Client::builder()
            .user_agent(user_agent)
            .build()?;

        Ok(Self {
            client,
            client_id,
            client_secret,
            user_agent: user_agent.to_string(),
            token: tokio::sync::Mutex::new(None),
        })
    }

    /// Get a valid OAuth2 access token (cached).
    async fn get_token(&self) -> Result<String> {
        let mut guard = self.token.lock().await;

        // Return cached token if available
        if let Some(tok) = guard.as_ref() {
            return Ok(tok.clone());
        }

        if self.client_id.is_empty() || self.client_secret.is_empty() {
            return Err(anyhow!("REDDIT_CLIENT_ID / REDDIT_CLIENT_SECRET not set"));
        }

        let resp: TokenResponse = self.client
            .post("https://www.reddit.com/api/v1/access_token")
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[("grant_type", "client_credentials")])
            .header("User-Agent", &self.user_agent)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let token = resp.access_token.clone();
        *guard = Some(resp.access_token);
        Ok(token)
    }

    /// Fetch the top `limit` hot posts from a subreddit.
    /// Tries OAuth2 first; falls back to anonymous JSON if credentials missing.
    pub async fn fetch_hot_posts(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        debug!("Fetching r/{} (limit {})", subreddit, limit);

        let posts = if !self.client_id.is_empty() && !self.client_secret.is_empty() {
            self.fetch_oauth(subreddit, limit).await?
        } else {
            self.fetch_anonymous(subreddit, limit).await?
        };

        debug!("Got {} usable posts from r/{}", posts.len(), subreddit);
        Ok(posts)
    }

    /// Fetch via Reddit OAuth2 API (oauth.reddit.com) — works reliably.
    async fn fetch_oauth(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        let token = match self.get_token().await {
            Ok(t) => t,
            Err(e) => {
                warn!("OAuth token fetch failed, trying anonymous: {}", e);
                // Invalidate cached token
                *self.token.lock().await = None;
                return self.fetch_anonymous(subreddit, limit).await;
            }
        };

        let url = format!(
            "https://oauth.reddit.com/r/{}/hot?limit={}&raw_json=1",
            subreddit, limit
        );

        let resp = self.client
            .get(&url)
            .bearer_auth(&token)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        // Token may have expired — retry once
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            *self.token.lock().await = None;
            return Box::pin(self.fetch_oauth(subreddit, limit)).await;
        }

        let response = resp
            .error_for_status()?
            .json::<RedditResponse>()
            .await?;

        Ok(self.filter_posts(response))
    }

    /// Anonymous fallback — uses a browser-like UA to avoid 403s.
    async fn fetch_anonymous(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        let url = format!(
            "https://www.reddit.com/r/{}/hot.json?limit={}&raw_json=1",
            subreddit, limit
        );

        let response = self.client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .header("Accept", "application/json")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await?
            .error_for_status()?
            .json::<RedditResponse>()
            .await?;

        Ok(self.filter_posts(response))
    }

    fn filter_posts(&self, response: RedditResponse) -> Vec<RedditPost> {
        response
            .data
            .children
            .into_iter()
            .map(|w| w.data)
            .filter(|p| !p.stickied)
            .collect()
    }

    /// Derive the best embeddable media URL for a post.
    ///
    /// Priority:
    ///   1. Reddit-hosted video fallback URL (mp4)
    ///   2. `url_overridden_by_dest` / `url` if it looks like a direct media file
    ///   3. Preview image source (HTML-decoded)
    ///   4. `None` — text / link post with no embeddable media
    pub fn media_url(post: &RedditPost) -> Option<String> {
        // 1. Reddit video
        if post.is_video {
            if let Some(media) = &post.media {
                if let Some(video) = &media.reddit_video {
                    let url = video
                        .fallback_url
                        .split('?')
                        .next()
                        .unwrap_or(&video.fallback_url)
                        .to_string();
                    return Some(url);
                }
            }
        }

        // 2. Direct media URL
        let raw_url = post
            .url_overridden_by_dest
            .as_deref()
            .unwrap_or(post.url.as_str());

        let lower = raw_url.to_lowercase();
        let is_direct_media = lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".png")
            || lower.ends_with(".gif")
            || lower.ends_with(".gifv")
            || lower.ends_with(".mp4")
            || lower.ends_with(".webp")
            || raw_url.contains("i.redd.it")
            || raw_url.contains("i.imgur.com")
            || raw_url.contains("preview.redd.it");

        if is_direct_media {
            return Some(raw_url.to_string());
        }

        // Check post_hint
        if let Some(hint) = &post.post_hint {
            if matches!(hint.as_str(), "image" | "hosted:video" | "rich:video") {
                return Some(raw_url.to_string());
            }
        }

        // 3. Preview image (HTML-encoded ampersands must be decoded)
        if let Some(preview) = &post.preview {
            if let Some(first) = preview.images.first() {
                let decoded = first.source.url.replace("&amp;", "&");
                return Some(decoded);
            }
        }

        warn!("No embeddable media found for post {}", post.id);
        None
    }
}
