use anyhow::Result;
use reqwest::Client;

use super::models::{IndianSearchResponse, IndianVideo};

const REDTUBE_API: &str = "https://api.redtube.com/";

/// Curated Indian viral / MMS search queries rotated across ticks.
pub const INDIAN_SEARCHES: &[&str] = &[
    "indian viral",
    "desi viral",
    "indian mms",
    "desi mms",
    "indian bhabhi",
    "desi bhabhi",
    "indian college girl",
    "desi couple",
    "indian sex scandal",
    "desi leak",
    "mallu aunty",
    "tamil desi mms",
    "punjabi mms",
    "pakistani mms",
    "indian romantic couple",
];

pub const INDIAN_KEYWORDS: &[&str] = &[
    "indian", "desi", "bhabhi", "tamil", "telugu", "mallu", "kannada",
    "hindi", "punjabi", "pakistani", "bengali", "marathi", "gujarati",
    "mms", "chut", "devar", "saali", "aunty", "aunties", "chachi",
    "viral", "leaked", "scandal",
];

pub struct IndianClient {
    http: Client,
}

impl IndianClient {
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

    /// Fetch Indian viral MMS videos via RedTube Webmasters REST API
    pub async fn fetch_indian_videos(&self, query: &str, count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        let resp = self
            .http
            .get(REDTUBE_API)
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
            .json::<IndianSearchResponse>()
            .await?;

        let all: Vec<IndianVideo> = resp.videos.into_iter().map(|w| w.video).collect();
        let filtered: Vec<IndianVideo> = all
            .iter()
            .filter(|v| {
                let lower = v.title.to_lowercase();
                INDIAN_KEYWORDS.iter().any(|k| lower.contains(k))
            })
            .cloned()
            .collect();

        if !filtered.is_empty() {
            Ok(filtered)
        } else {
            Ok(all)
        }
    }
}
