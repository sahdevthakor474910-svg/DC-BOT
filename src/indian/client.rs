use anyhow::Result;

use super::models::IndianVideo;

/// Curated authentic Indian viral MMS search queries rotated across ticks.
pub const INDIAN_SEARCHES: &[&str] = &[
    "hot college girl mms",
    "college girl mms",
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

    /// Fetch authentic Indian viral MMS videos from XNXX with OK.XXX fallback.
    pub async fn fetch_indian_videos(&self, query: &str, _count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        let xnxx_videos = self.xnxx.search_videos(query, page).await.unwrap_or_default();

        let mut all: Vec<IndianVideo> = xnxx_videos
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

        // If XNXX returns 0 videos or times out, seamlessly fallback to OK.XXX
        if all.is_empty() {
            if let Ok(ok_client) = crate::okxxx::client::OkXxxClient::new() {
                let ok_query = if query.contains("college") {
                    "college-girl"
                } else {
                    "indian"
                };
                if let Ok(ok_videos) = ok_client.search_videos(ok_query, page).await {
                    for ov in ok_videos {
                        all.push(IndianVideo {
                            id: format!("ok_{}", ov.video_id),
                            title: ov.title,
                            duration: ov.duration,
                            views: serde_json::Value::String(ov.views),
                            default_thumb: ov.thumbnail,
                            url: ov.url,
                        });
                    }
                }
            }
        }

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
                    || lower.contains("aunty")
                    || lower.contains("aunt")
                    || lower.contains("step");
                if is_bhabhi_or_scripted {
                    return false;
                }

                // Strictly require MMS or college girl in title
                lower.contains("mms") || (lower.contains("college") && lower.contains("girl"))
            })
            .collect();

        Ok(filtered)
    }

    /// Resolve playable direct MP4 for an Indian video.
    pub async fn get_mp4_url(&self, video_url_or_id: &str) -> Result<String> {
        if video_url_or_id.contains("ok.xxx") || video_url_or_id.starts_with("ok_") {
            let ok_client = crate::okxxx::client::OkXxxClient::new()?;
            let full_url = if video_url_or_id.starts_with("http") {
                video_url_or_id.to_string()
            } else {
                let id = video_url_or_id.trim_start_matches("ok_");
                format!("https://ok.xxx/video/{}/", id)
            };
            ok_client.get_mp4_url(&full_url).await
        } else if video_url_or_id.contains("xnxx.com") || video_url_or_id.contains("video-") {
            let full_url = if video_url_or_id.starts_with("http") {
                video_url_or_id.to_string()
            } else {
                format!("https://www.xnxx.com/video-{}", video_url_or_id)
            };
            match self.xnxx.get_mp4_url(&full_url).await {
                Ok(url) if !url.is_empty() => Ok(url),
                _ => {
                    // Fallback to okxxx if xnxx times out or fails
                    let ok_client = crate::okxxx::client::OkXxxClient::new()?;
                    let search_res = ok_client.search_videos("indian", 1).await?;
                    if let Some(first) = search_res.first() {
                        ok_client.get_mp4_url(&first.url).await
                    } else {
                        anyhow::bail!("No stream found on XNXX or OK.XXX")
                    }
                }
            }
        } else {
            let clean_id = video_url_or_id.trim_start_matches("ok_");
            let porn_client = crate::porn::client::PornClient::new()?;
            porn_client.get_mp4_url(clean_id).await
        }
    }
}

