use anyhow::Result;

use super::models::IndianVideo;

/// Curated authentic Indian viral MMS search queries rotated across ticks.
pub const INDIAN_SEARCHES: &[&str] = &[
    "viral mms",
    "indian viral mms",
    "indian mms viral",
    "desi mms viral",
    "indian mms video",
    "college mms",
    "real mms",
    "leaked mms",
    "homemade mms",
    "indian mms leaked",
    "desi mms leak",
    "mms video",
    "indian college mms",
    "desi scandal mms",
    "hindi viral mms",
    "mallu viral mms",
    "tamil viral mms",
    "punjabi viral mms",
];

pub struct IndianClient {
    xnxx: crate::xnxx::client::XnxxClient,
}

impl IndianClient {
    pub fn new() -> Result<Self> {
        let xnxx = crate::xnxx::client::XnxxClient::new()?;
        Ok(Self { xnxx })
    }

    /// Fetch authentic Indian viral MMS videos from XNXX.
    pub async fn fetch_indian_videos(&self, query: &str, _count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        let xnxx_videos = self.xnxx.search_videos(query, page).await?;

        let all: Vec<IndianVideo> = xnxx_videos
            .into_iter()
            .map(|x| IndianVideo {
                id: x.video_id,
                title: x.title,
                duration: x.duration,
                views: serde_json::Value::String(x.views),
                default_thumb: x.thumbnail,
                url: x.url,
            })
            .collect();

        let filtered: Vec<IndianVideo> = all
            .into_iter()
            .filter(|v| {
                let lower = v.title.to_lowercase();
                let is_unwanted = lower.contains("native american")
                    || lower.contains("indianapolis")
                    || lower.contains("indiana ")
                    || lower.contains("apache")
                    || lower.contains("navajo")
                    || lower.contains("cherokee")
                    || lower.contains("feather")
                    || lower.contains("swiss resort");
                if is_unwanted {
                    return false;
                }

                // Strictly remove bhabhi and family roleplay scripted tags per user request
                let is_bhabhi_or_scripted = lower.contains("bhabhi")
                    || lower.contains("bhabi")
                    || lower.contains("devar")
                    || lower.contains("saali")
                    || lower.contains("sali ")
                    || lower.contains("chachi")
                    || lower.contains("sasur")
                    || lower.contains("bahu")
                    || lower.contains("jija")
                    || lower.contains("step");
                if is_bhabhi_or_scripted {
                    return false;
                }

                // Strictly require MMS in title
                lower.contains("mms")
            })
            .collect();

        Ok(filtered)
    }

    /// Resolve playable direct MP4 for an Indian video.
    pub async fn get_mp4_url(&self, video_url_or_id: &str) -> Result<String> {
        if video_url_or_id.contains("xnxx.com") || video_url_or_id.contains("video-") {
            let full_url = if video_url_or_id.starts_with("http") {
                video_url_or_id.to_string()
            } else {
                format!("https://www.xnxx.com/video-{}", video_url_or_id)
            };
            self.xnxx.get_mp4_url(&full_url).await
        } else {
            let clean_id = video_url_or_id.trim_start_matches("ok_");
            let porn_client = crate::porn::client::PornClient::new()?;
            porn_client.get_mp4_url(clean_id).await
        }
    }
}

