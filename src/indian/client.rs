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
    http: reqwest::Client,
    xnxx: crate::xnxx::client::XnxxClient,
}

impl IndianClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(12))
            .build()?;
        let xnxx = crate::xnxx::client::XnxxClient::new()?;
        Ok(Self { http, xnxx })
    }

    /// Fetch authentic Indian viral MMS videos combining RedTube API, OK.XXX, and XNXX.
    pub async fn fetch_indian_videos(&self, query: &str, count: u32, page: u32) -> Result<Vec<IndianVideo>> {
        let mut all: Vec<IndianVideo> = Vec::new();

        // 1. Fetch from RedTube API (tens of thousands of tagged videos)
        let count_str = std::cmp::max(count, 12).to_string();
        let page_str = page.to_string();
        if let Ok(resp) = self
            .http
            .get("https://api.redtube.com/")
            .query(&[
                ("data", "redtube.Videos.searchVideos"),
                ("output", "json"),
                ("search", query),
                ("thumbsize", "medium2"),
                ("ordering", "most_recent"),
                ("count", &count_str),
                ("page", &page_str),
            ])
            .send()
            .await
        {
            if let Ok(search_resp) = resp.json::<super::models::IndianSearchResponse>().await {
                for w in search_resp.videos {
                    all.push(w.video);
                }
            }
        }

        // 2. Fetch from OK.XXX
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

        // 3. Fetch from XNXX
        if let Ok(xnxx_videos) = self.xnxx.search_videos(query, page).await {
            for xv in xnxx_videos {
                all.push(IndianVideo {
                    id: xv.video_id,
                    title: xv.title,
                    duration: xv.duration,
                    views: serde_json::Value::String(xv.views),
                    default_thumb: xv.thumbnail,
                    url: xv.url,
                });
            }
        }

        // Deduplicate by ID
        let mut seen_ids = std::collections::HashSet::new();
        all.retain(|v| seen_ids.insert(v.id.clone()));

        // Filter strictly
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
                    || lower.contains("step")
                    || lower.contains("family");
                if is_bhabhi_or_scripted {
                    return false;
                }

                true
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
            let video_id = crate::web::extract_redtube_id(video_url_or_id)
                .unwrap_or_else(|| video_url_or_id.to_string());
            let porn_client = crate::porn::client::PornClient::new()?;
            match porn_client.get_mp4_url(&video_id).await {
                Ok(url) if !url.is_empty() => Ok(url),
                _ => {
                    // Fallback to OK.XXX
                    let ok_client = crate::okxxx::client::OkXxxClient::new()?;
                    let search_res = ok_client.search_videos("indian", 1).await?;
                    if let Some(first) = search_res.first() {
                        ok_client.get_mp4_url(&first.url).await
                    } else {
                        anyhow::bail!("No stream found on Redtube or OK.XXX")
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_college_girl_mms() {
        let client = IndianClient::new().expect("Failed to initialize IndianClient");
        let videos = client
            .fetch_indian_videos("hot college girl mms", 8, 1)
            .await
            .expect("fetch_indian_videos failed");
        println!("Fetched {} Indian MMS videos", videos.len());
        assert!(!videos.is_empty(), "Should fetch at least 1 video");
        for v in &videos {
            let lower = v.title.to_lowercase();
            assert!(!lower.contains("bhabhi"), "Must not contain bhabhi: {}", v.title);
            assert!(!lower.contains("devar"), "Must not contain devar: {}", v.title);
            assert!(!lower.contains("saali"), "Must not contain saali: {}", v.title);
            println!("- [{}] {} ({})", v.id, v.title, v.url);
        }
    }
}

