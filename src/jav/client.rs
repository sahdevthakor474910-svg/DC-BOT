use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::Client;

use super::models::{EpornerSearchResponse, EpornerVideo, EpornerVideoEntry};

const EPORNER_API: &str = "https://www.eporner.com/api/v2/video/search/";

/// JAV-specific search queries rotated each tick.
pub const JAV_SEARCHES: &[&str] = &[
    "japanese uncensored",
    "jav uncensored",
    "japanese amateur",
    "asian uncensored",
    "japanese milf",
    "jav subtitled",
    "japanese teen",
    "asian milf uncensored",
];

pub struct EpornerClient {
    http: Client,
}

impl EpornerClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { http })
    }

    /// Search for videos using eporner's Webmasters API.
    /// Returns raw entries (no MP4 URL yet — need to scrape page).
    pub async fn search(&self, query: &str, count: u32) -> Result<Vec<EpornerVideoEntry>> {
        let resp = self
            .http
            .get(EPORNER_API)
            .query(&[
                ("query", query),
                ("per_page", &count.to_string()),
                ("format", "json"),
                ("order", "top-weekly"),
                ("gay", "0"),
                ("thumbsize", "big"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<EpornerSearchResponse>()
            .await?;

        Ok(resp.videos)
    }

    /// Fetch the video page HTML and extract the direct CDN `.mp4` URL.
    ///
    /// eporner stores video sources in a JSON blob inside a `<script>` tag:
    /// `{"src":"https://cdn3.eporner.com/...720.mp4","res":720}` etc.
    /// We pick the highest-res source ≤ 720p so files aren't massive.
    pub async fn get_mp4_url(&self, video_id: &str) -> Result<String> {
        // eporner page URLs are: /video-XXXXXXXX/anything/
        let page_url = format!("https://www.eporner.com/video-{}/jav/", video_id);
        let html = self
            .http
            .get(&page_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // Extract the JSON sources array from the player config in the page.
        // Pattern: {"src":"https://...mp4","res":NUM,"type":"..."}
        let re = Regex::new(r#"\{"src":"(https://[^"]+\.mp4)","res":(\d+)"#)
            .map_err(|e| anyhow!("Regex error: {}", e))?;

        let mut best: Option<(u32, String)> = None;
        for cap in re.captures_iter(&html) {
            let src = cap[1].to_string();
            let res: u32 = cap[2].parse().unwrap_or(0);
            // Pick highest quality ≤ 720p (keep file size Discord-friendly)
            if res <= 720 {
                if best.as_ref().map_or(true, |(r, _)| res > *r) {
                    best = Some((res, src));
                }
            }
        }

        match best {
            Some((_, url)) => Ok(url),
            None => Err(anyhow!("No suitable MP4 source found on eporner video page for id={}", video_id)),
        }
    }

    /// Full pipeline: search → for each result, resolve MP4 URL.
    /// Returns only videos where MP4 extraction succeeded.
    pub async fn fetch_jav_videos(&self, query: &str, count: u32) -> Result<Vec<EpornerVideo>> {
        let entries = self.search(query, count).await?;
        let mut results = Vec::new();

        for entry in entries {
            // Build page URL
            let slug = slug_from_title(&entry.title);
            let page_url = format!("https://www.eporner.com/video-{}/{}/", entry.id, slug);

            // Pick best thumbnail
            let thumb_url = best_thumb(&entry);

            let mp4_url = match self.get_mp4_url(&entry.id).await {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("Could not get MP4 for eporner id={}: {}", entry.id, e);
                    continue;
                }
            };

            results.push(EpornerVideo {
                id: entry.id,
                title: entry.title,
                page_url,
                mp4_url,
                thumb_url,
                duration: entry.length_min.unwrap_or_else(|| entry.length.unwrap_or_default()),
                views: entry.views.unwrap_or_default(),
            });
        }

        Ok(results)
    }
}

/// Convert a title to a URL-friendly slug (alphanumeric + hyphens).
fn slug_from_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(50)
        .collect()
}

/// Pick the largest available thumbnail from the thumbs array.
fn best_thumb(entry: &EpornerVideoEntry) -> String {
    if let Some(thumbs) = &entry.thumbs {
        if let Some(best) = thumbs.iter().max_by_key(|t| t.width.unwrap_or(0)) {
            return best.src.clone();
        }
    }
    String::new()
}
