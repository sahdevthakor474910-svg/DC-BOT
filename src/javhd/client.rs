use anyhow::Result;
use reqwest::Client;
use super::models::{JavhdApiResponse, JavhdVideo};

const BASE_URL: &str = "https://javhd.com";

/// API endpoints to rotate through each tick for variety
const API_CONFIGS: &[(bool, u32, u32)] = &[
    // (isCasting, count, offset)
    (true,  10, 0),   // Homepage featured
    (false, 10, 21),  // Recently added  
    (false, 10, 25),  // Most popular
    (true,  10, 10),  // Page 2
    (false, 10, 30),  // Page 3
];

pub struct JavhdClient {
    http: Client,
}

impl JavhdClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { http })
    }

    /// Fetch videos from JAVHD API for a given tick (rotates through configs)
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<JavhdVideo>> {
        let idx = (tick as usize) % API_CONFIGS.len();
        let (is_casting, count, offset) = API_CONFIGS[idx];
        self.fetch_videos(is_casting, count, offset).await
    }

    async fn fetch_videos(&self, is_casting: bool, count: u32, offset: u32) -> Result<Vec<JavhdVideo>> {
        let casting = if is_casting { "1" } else { "0" };
        let url = format!(
            "{}/en/api/content_block?block=custom&pgid=807501519&isCasting={}&count={}&offset={}",
            BASE_URL, casting, count, offset
        );

        let resp = self.http
            .get(&url)
            .header("Referer", "https://javhd.com/en/")
            .send()
            .await?
            .error_for_status()?
            .json::<JavhdApiResponse>()
            .await?;

        let videos: Vec<JavhdVideo> = resp.template
            .into_iter()
            .filter_map(|entry| {
                let preview = entry.video_medium
                    .or(entry.video)
                    .unwrap_or_default();
                if preview.is_empty() {
                    return None;
                }

                let page_url = entry.studio_url
                    .map(|u| format!("{}{}", BASE_URL, u))
                    .unwrap_or_else(|| format!("{}/en/id/{}", BASE_URL, entry.id));

                // Pick best thumbnail (largest resolution)
                let thumb_url = entry.thumbs
                    .get("940x530")
                    .or_else(|| entry.thumbs.get("1130x706"))
                    .or_else(|| entry.thumbs.get("468x264"))
                    .or_else(|| entry.thumbs.get("374x233"))
                    .cloned()
                    .unwrap_or_default();

                Some(JavhdVideo {
                    id: entry.id.to_string(),
                    title: entry.title,
                    page_url,
                    preview_mp4: preview,
                    thumb_url,
                    duration: entry.length.unwrap_or_else(|| "?".to_string()),
                    views: entry.clicks.unwrap_or(0),
                })
            })
            .collect();

        Ok(videos)
    }
}
