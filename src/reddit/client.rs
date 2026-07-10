use anyhow::Result;
use reqwest::Client;
use tracing::{debug, warn};

use super::models::{RedditPost, RedditResponse};

/// Subreddits to poll for memes.
pub const SUBREDDITS: &[&str] = &["memes", "dankmemes", "shitposting", "brainrot", "196", "whenthe"];

// ────────────────────────────────────────────────────────────────────────────

pub struct RedditClient {
    client: Client,
}

impl RedditClient {
    pub fn new(user_agent: &str) -> Result<Self> {
        let client = Client::builder()
            .user_agent(user_agent)
            .build()?;
        Ok(Self { client })
    }

    /// Fetch the top `limit` hot posts from a subreddit.
    /// Stickied posts are filtered; NSFW posts are included.
    pub async fn fetch_hot_posts(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        let url = format!(
            "https://www.reddit.com/r/{}/hot.json?limit={}&raw_json=1",
            subreddit, limit
        );

        debug!("Fetching r/{} (limit {})", subreddit, limit);

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<RedditResponse>()
            .await?;

        let posts: Vec<RedditPost> = response
            .data
            .children
            .into_iter()
            .map(|w| w.data)
            .filter(|p| !p.stickied)
            .collect();

        debug!("Got {} usable posts from r/{}", posts.len(), subreddit);
        Ok(posts)
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
                    // Remove the query params Reddit appends (some cause embed failures)
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
