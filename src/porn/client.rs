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

    /// Scrape the Redtube video page to find the direct playable .mp4 URL.
    pub async fn get_mp4_url(&self, video_id: &str) -> Result<String> {
        let page_url = format!("https://www.redtube.com/{}", video_id);
        let resp = self.http.get(&page_url)
            .header("Referer", "https://www.redtube.com/")
            .send()
            .await?;
        
        let html = resp.error_for_status()?.text().await?;

        // 1. Try mediaDefinition JSON array
        let media_def_re = regex::Regex::new(r#"mediaDefinition\s*:\s*(\[[^\]]+\])"#)?;
        if let Some(cap) = media_def_re.captures(&html) {
            let json_str = &cap[1];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(arr) = val.as_array() {
                    let mut best: Option<(u32, String)> = None;
                    for item in arr {
                        let video_url_val = item.get("videoUrl").and_then(|v| v.as_str()).unwrap_or_default();
                        if video_url_val.is_empty() {
                            continue;
                        }
                        let _format = item.get("format").and_then(|v| v.as_str()).unwrap_or("");
                        let quality_str = item.get("quality").and_then(|v| v.as_str())
                            .or_else(|| item.get("videoUrl").and_then(|v| v.as_str()).and_then(|u| {
                                if u.contains("_720p") { Some("720") }
                                else if u.contains("_480p") { Some("480") }
                                else if u.contains("_360p") { Some("360") }
                                else { None }
                            }))
                            .unwrap_or("");
                        let quality: u32 = quality_str.parse().unwrap_or(0);

                        // If videoUrl does not point to mp4 directly, it could be a JSON endpoint
                        let direct_url = if video_url_val.contains("ht_json") || video_url_val.contains("/api/player") || quality == 0 {
                            let fetch_url = if video_url_val.starts_with('/') {
                                format!("https://www.redtube.com{}", video_url_val)
                            } else {
                                video_url_val.to_string()
                            };
                            if let Ok(xhr_resp) = self.http.get(&fetch_url).header("Referer", &page_url).send().await {
                                if let Ok(xhr_json) = xhr_resp.json::<serde_json::Value>().await {
                                    let mut resolved = None;
                                    if let Some(xhr_arr) = xhr_json.as_array() {
                                        for entry in xhr_arr {
                                            if let Some(u) = entry.get("videoUrl").and_then(|v| v.as_str()) {
                                                if !u.is_empty() {
                                                    resolved = Some(u.to_string());
                                                    break;
                                                }
                                            }
                                        }
                                    } else if let Some(u) = xhr_json.get("videoUrl").and_then(|v| v.as_str()) {
                                        resolved = Some(u.to_string());
                                    }
                                    resolved
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            Some(video_url_val.to_string())
                        };

                        if let Some(du) = direct_url {
                            if !du.is_empty() && !du.contains("m3u8") {
                                if quality <= 720 {
                                    if best.as_ref().map_or(true, |(q, _)| quality > *q || *q == 0) {
                                        best = Some((quality, du));
                                    }
                                }
                            }
                        }
                    }
                    if let Some((_, u)) = best {
                        return Ok(u);
                    }
                }
            }
        }

        // 2. Try HTML source tag regex fallback
        let source_re = regex::Regex::new(r#"<source[^>]+src=["']([^"']+\.mp4(?:\?[^"']+)?)["']"#)?;
        if let Some(cap) = source_re.captures(&html) {
            return Ok(cap[1].to_string());
        }

        // 3. Try plain text .mp4 inside javascript sources
        let sources_re = regex::Regex::new(r#"sources\s*:\s*(\{[^}]+\})"#)?;
        if let Some(cap) = sources_re.captures(&html) {
            let inner = &cap[1];
            let mp4_find = regex::Regex::new(r#""([^"]+\.mp4[^"]*)""#)?;
            if let Some(m) = mp4_find.captures(inner) {
                return Ok(m[1].to_string().replace("\\/", "/"));
            }
        }

        Err(anyhow::anyhow!("No direct MP4 URL found for Redtube video ID {}", video_id))
    }
}
