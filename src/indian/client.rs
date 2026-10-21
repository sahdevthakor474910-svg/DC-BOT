use anyhow::Result;
use reqwest::Client;

use super::models::IndianVideo;

/// Curated Indian viral / MMS search queries rotated across ticks.
pub const INDIAN_SEARCHES: &[&str] = &[
    "indian",
    "desi",
    "bhabhi",
    "indian viral",
    "desi mms",
    "desi bhabhi",
    "mallu",
    "tamil",
    "punjabi",
    "hindi",
    "desi couple",
    "indian aunty",
    "pakistani",
    "indian romantic",
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

    /// Fetch Indian viral MMS videos.
    /// Primary engine: Ok.xxx (guaranteed direct 720p/1080p MP4 streams with 0 ISP blocks).
    /// Fallback engine: RedTube Webmasters API.
    pub async fn fetch_indian_videos(&self, query: &str, _count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        // 1. Try Ok.xxx search first (ultra-fast, unblocked in India, direct MP4)
        if let Ok(ok_client) = crate::okxxx::client::OkXxxClient::new() {
            let words: Vec<&str> = query.split_whitespace().collect();
            let primary_tag = if words.contains(&"desi") {
                "desi"
            } else if words.contains(&"bhabhi") {
                "bhabhi"
            } else if words.contains(&"mms") {
                "mms"
            } else if words.contains(&"mallu") {
                "mallu"
            } else if words.contains(&"tamil") {
                "tamil"
            } else if words.contains(&"punjabi") {
                "punjabi"
            } else if words.contains(&"hindi") {
                "hindi"
            } else if words.contains(&"pakistani") {
                "pakistani"
            } else {
                words.first().copied().unwrap_or("indian")
            };

            if let Ok(ok_vids) = ok_client.search_videos(primary_tag, page).await {
                if !ok_vids.is_empty() {
                    let mut list: Vec<IndianVideo> = ok_vids
                        .into_iter()
                        .map(|v| IndianVideo {
                            id: format!("ok_{}", v.video_id),
                            title: v.title,
                            duration: v.duration,
                            views: serde_json::Value::String(v.views),
                            default_thumb: v.thumbnail,
                            url: v.url,
                        })
                        .collect();

                    if words.len() > 1 {
                        let secondary: Vec<&str> = words.iter().skip(1).copied().collect();
                        let filtered: Vec<IndianVideo> = list
                            .iter()
                            .filter(|v| {
                                let lower = v.title.to_lowercase();
                                secondary.iter().any(|s| lower.contains(s))
                            })
                            .cloned()
                            .collect();
                        if !filtered.is_empty() {
                            list = filtered;
                        }
                    }

                    return Ok(list);
                }
            }
        }

        // 2. Fallback to RedTube API
        let resp = self
            .http
            .get("https://api.redtube.com/")
            .query(&[
                ("data", "redtube.Videos.searchVideos"),
                ("output", "json"),
                ("search", query),
                ("thumbsize", "medium2"),
                ("ordering", "most_recent"),
                ("count", &"20"),
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

    /// Resolve playable direct MP4 URL for an Indian video.
    pub async fn get_mp4_url(&self, video_url_or_id: &str) -> Result<String> {
        if video_url_or_id.contains("ok.xxx") || video_url_or_id.starts_with("ok_") {
            let ok_client = crate::okxxx::client::OkXxxClient::new()?;
            let page_url = if video_url_or_id.starts_with("http") {
                video_url_or_id.to_string()
            } else {
                let num_id = video_url_or_id.trim_start_matches("ok_");
                format!("https://ok.xxx/video/{}/", num_id)
            };
            return ok_client.get_mp4_url(&page_url).await;
        }

        let id = video_url_or_id.trim_start_matches("ok_");
        let porn_client = crate::porn::client::PornClient::new()?;
        porn_client.get_mp4_url(id).await
    }
}
