use anyhow::Result;
use reqwest::Client;

use super::models::{JavSearchResponse, JavVideo};

const JAV_API: &str = "https://api.redtube.com/";

/// Curated JAV search queries rotated across ticks.
pub const JAV_SEARCHES: &[&str] = &[
    "japanese jav",
    "jav uncensored",
    "japanese uncensored",
    "asian jav",
    "japanese massage",
    "japanese creampie",
    "tokyo hot",
    "caribbeancom",
    "1pondo",
    "heyzo",
    "japanese schoolgirl",
    "japanese teen",
    "japanese milf",
];

pub struct JavClient {
    http: Client,
}

impl JavClient {
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

    /// Fetch Japanese/JAV videos via free Webmasters REST API
    pub async fn fetch_jav_videos(&self, query: &str, count: u32, page: u32) -> Result<Vec<JavVideo>> {
        let resp = self
            .http
            .get(JAV_API)
            .query(&[
                ("data", "redtube.Videos.searchVideos"),
                ("output", "json"),
                ("search", query),
                ("thumbsize", "medium2"),
                ("ordering", "most_recent"),
                ("count", &count.to_string()),
                ("page", &page.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<JavSearchResponse>()
            .await?;

        Ok(resp.videos.into_iter().map(|w| w.video).collect())
    }
}
