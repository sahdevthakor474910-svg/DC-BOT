use anyhow::Result;
use reqwest::Client;

use super::models::{RedTubeResponse, RedTubeVideo};

const REDTUBE_API: &str = "https://api.redtube.com/";

/// Search categories we rotate through every tick.
/// Each search maps to a different vibe — studio, category, or keyword.
pub const PORN_SEARCHES: &[&str] = &[
    "naughty america",
    "brazzers",
    "milf",
    "step mom",
    "teen",
    "lesbian",
    "amateur",
    "big tits",
    "asian",
    "blonde",
    "anal",
    "threesome",
];

pub struct PornClient {
    http: Client,
}

impl PornClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self { http })
    }

    /// Fetch the most recent videos for a given search term.
    pub async fn fetch_videos(&self, search: &str, count: u8) -> Result<Vec<RedTubeVideo>> {
        let resp = self
            .http
            .get(REDTUBE_API)
            .query(&[
                ("data", "redtube.Videos.searchVideos"),
                ("output", "json"),
                ("search", search),
                ("thumbsize", "medium2"),   // 320x240 — good for Discord embed
                ("ordering", "most_recent"),
                ("count", &count.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<RedTubeResponse>()
            .await?;

        Ok(resp.videos.into_iter().map(|w| w.video).collect())
    }

    /// Fetch top-rated videos of all time (for a variety feed).
    pub async fn fetch_top_rated(&self, count: u8) -> Result<Vec<RedTubeVideo>> {
        let resp = self
            .http
            .get(REDTUBE_API)
            .query(&[
                ("data", "redtube.Videos.searchVideos"),
                ("output", "json"),
                ("thumbsize", "medium2"),
                ("ordering", "rating"),
                ("count", &count.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<RedTubeResponse>()
            .await?;

        Ok(resp.videos.into_iter().map(|w| w.video).collect())
    }
}
