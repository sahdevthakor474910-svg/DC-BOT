use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

use super::models::RedditPost;

/// Subreddits to poll for memes.
pub const SUBREDDITS: &[&str] = &["memes", "dankmemes", "shitposting", "brainrot", "196", "whenthe"];

/// NSFW subreddits — only posted to age-restricted NSFW channels.
pub const NSFW_SUBREDDITS: &[&str] = &["nsfw", "gonewild", "rule34", "hentai", "porn"];

// ─── Meme API response ────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct MemeApiResponse {
    #[serde(default)]
    memes: Vec<MemeApiPost>,
    // Single meme response fields
    postLink: Option<String>,
    subreddit: Option<String>,
    title: Option<String>,
    url: Option<String>,
    nsfw: Option<bool>,
    spoiler: Option<bool>,
    author: Option<String>,
    ups: Option<u32>,
    preview: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct MemeApiPost {
    postLink: String,
    subreddit: String,
    title: String,
    url: String,
    nsfw: bool,
    spoiler: bool,
    author: String,
    ups: u32,
    preview: Vec<String>,
}

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

    /// Fetch the top posts from a subreddit using meme-api.com (no auth needed).
    pub async fn fetch_hot_posts(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        debug!("Fetching r/{} via meme-api.com (limit {})", subreddit, limit);

        let url = format!("https://meme-api.com/gimme/{}/{}", subreddit, limit);

        let response = self.client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<MemeApiResponse>()
            .await?;

        let posts: Vec<RedditPost> = response.memes
            .into_iter()
            .filter(|m| !m.url.is_empty())
            .map(|m| {
                // postLink is like "https://redd.it/1uskdj5" — the ID is the last path segment
                let id = m.postLink
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(&m.postLink)
                    .to_string();

                RedditPost {
                    id,
                    title: m.title,
                    author: m.author,
                    score: m.ups as i64,
                    url: m.url.clone(),
                    url_overridden_by_dest: Some(m.url),
                    is_video: false,
                    over_18: m.nsfw,
                    spoiler: m.spoiler,
                    stickied: false,
                    post_hint: Some("image".to_string()),
                    media: None,
                    preview: None,
                    subreddit: m.subreddit,
                    // store the full postLink as permalink so embed URL works
                    permalink: m.postLink,
                }
            })
            .collect();

        debug!("Got {} posts from r/{} via meme-api", posts.len(), subreddit);
        Ok(posts)
    }

    /// Derive the best embeddable media URL for a post.
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

        // 3. Preview image
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
