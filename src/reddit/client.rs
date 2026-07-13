use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};


use super::models::{RedditPost, RedditResponse};

/// Subreddits to poll for memes.
pub const SUBREDDITS: &[&str] = &["memes", "dankmemes", "shitposting", "brainrot", "196", "whenthe"];

/// NSFW subreddits — only posted to age-restricted NSFW channels.
pub const NSFW_SUBREDDITS: &[&str] = &["nsfw", "gonewild", "rule34", "hentai", "RealGirls", "milf", "boobs", "amateur"];


// ─── Meme API response ────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct MemeApiResponse {
    #[serde(default)]
    memes: Vec<MemeApiPost>,
}

#[derive(Deserialize, Debug)]
struct MemeApiPost {
    #[serde(rename = "postLink")]
    post_link: String,
    subreddit: String,
    title: String,
    url: String,
    nsfw: bool,
    spoiler: bool,
    author: String,
    ups: u32,
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

    /// Fetch the top posts from a subreddit using meme-api.com with direct Reddit JSON fallback.
    pub async fn fetch_hot_posts(&self, subreddit: &str, limit: u32) -> Result<Vec<RedditPost>> {
        debug!("Fetching r/{} via meme-api.com (limit {})", subreddit, limit);

        let url = format!("https://meme-api.com/gimme/{}/{}", subreddit, limit);

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(resp) = resp.error_for_status() {
                    if let Ok(response) = resp.json::<MemeApiResponse>().await {
                        let posts: Vec<RedditPost> = response.memes
                            .into_iter()
                            .filter(|m| !m.url.is_empty())
                            .map(|m| {
                                let id = m.post_link
                                    .trim_end_matches('/')
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or(&m.post_link)
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
                                    permalink: m.post_link,
                                }
                            })
                            .collect();

                        debug!("Got {} posts from r/{} via meme-api", posts.len(), subreddit);
                        return Ok(posts);
                    }
                }
            }
            Err(e) => {
                warn!("Meme-api request error for r/{}: {}", subreddit, e);
            }
        }

        // ── Direct Reddit hot.json fallback ──────────────────────────────────────
        warn!("meme-api.com failed or returned 503 for r/{}. Falling back to direct Reddit hot.json API...", subreddit);
        let reddit_url = format!("https://www.reddit.com/r/{}/hot.json?limit={}", subreddit, limit);

        let response = self.client
            .get(&reddit_url)
            .send()
            .await?
            .error_for_status()?
            .json::<RedditResponse>()
            .await?;

        let mut posts = Vec::new();
        for child in response.data.children {
            let mut post = child.data;
            if post.url.is_empty() {
                continue;
            }
            // Ensure permalink is a full URL:
            if !post.permalink.starts_with("http") {
                post.permalink = format!("https://www.reddit.com{}", post.permalink);
            }
            posts.push(post);
        }

        debug!("Got {} posts from r/{} via direct Reddit fallback", posts.len(), subreddit);
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
