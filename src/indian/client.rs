use anyhow::Result;
use reqwest::Client;

use super::models::IndianVideo;

/// Curated Indian viral / MMS search queries rotated across ticks.
pub const INDIAN_SEARCHES: &[&str] = &[
    "indian viral",
    "indian mms",
    "desi mms",
    "indian viral mms",
    "desi bhabhi mms",
    "indian leaked mms",
    "desi viral mms",
    "indian college viral",
    "desi scandal mms",
    "hindi viral mms",
    "indian bhabhi viral",
    "mallu viral mms",
    "tamil viral mms",
    "punjabi viral mms",
    "desi leak viral",
];

pub const INDIAN_KEYWORDS: &[&str] = &[
    "indian", "desi", "bhabhi", "tamil", "telugu", "mallu", "kannada",
    "hindi", "punjabi", "pakistani", "bengali", "marathi", "gujarati",
    "mms", "chut", "devar", "saali", "aunty", "aunties", "chachi",
    "viral", "leaked", "scandal", "colleg", "leak",
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

    /// Fetch Indian viral MMS videos from RedTube Webmasters API.
    pub async fn fetch_indian_videos(&self, query: &str, count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        let fetch_count = count.max(20).to_string();
        let resp = self
            .http
            .get("https://api.redtube.com/")
            .query(&[
                ("data", "redtube.Videos.searchVideos"),
                ("output", "json"),
                ("search", query),
                ("thumbsize", "medium2"),
                ("ordering", "most_recent"),
                ("count", &fetch_count),
                ("page", &page.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<super::models::IndianSearchResponse>()
            .await?;

        let all: Vec<IndianVideo> = resp.videos.into_iter().map(|w| w.video).collect();
        let filtered: Vec<IndianVideo> = all
            .iter()
            .filter(|v| {
                let lower = v.title.to_lowercase();
                let is_unwanted = lower.contains("native american")
                    || lower.contains("indianapolis")
                    || lower.contains("indiana ")
                    || lower.contains("apache")
                    || lower.contains("navajo")
                    || lower.contains("cherokee")
                    || lower.contains("feather");
                if is_unwanted {
                    return false;
                }
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

    /// Resolve playable direct MP4 or HLS URL for an Indian video.
    pub async fn get_mp4_url(&self, video_url_or_id: &str) -> Result<String> {
        let clean_id = video_url_or_id.trim_start_matches("ok_");
        let porn_client = crate::porn::client::PornClient::new()?;
        porn_client.get_mp4_url(clean_id).await
    }
}

