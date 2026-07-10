use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::Client;

use serde::Deserialize;

use super::models::{EpornerSearchResponse, EpornerVideo, EpornerVideoEntry};

#[derive(Deserialize, Debug)]
struct XhrResponse {
    sources: Option<std::collections::HashMap<String, serde_json::Value>>,
}

const EPORNER_API: &str = "https://www.eporner.com/api/v2/video/search/";

/// JAV-specific search queries rotated each tick.
pub const JAV_SEARCHES: &[&str] = &[
    "japanese uncensored",
    "jav uncensored",
    "japanese amateur",
    "asian uncensored",
    "japanese milf",
    "jav subtitled",
    "japanese teen",
    "asian milf uncensored",
];

pub struct EpornerClient {
    http: Client,
}

impl EpornerClient {
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

    /// Search for videos using eporner's Webmasters API.
    /// Returns raw entries (no MP4 URL yet — need to scrape page).
    pub async fn search(&self, query: &str, count: u32) -> Result<Vec<EpornerVideoEntry>> {
        let resp = self
            .http
            .get(EPORNER_API)
            .query(&[
                ("query", query),
                ("per_page", &count.to_string()),
                ("format", "json"),
                ("order", "top-weekly"),
                ("gay", "0"),
                ("thumbsize", "big"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<EpornerSearchResponse>()
            .await?;

        Ok(resp.videos)
    }

    /// Fetch the video page HTML and extract the direct CDN `.mp4` URL.
    ///
    /// eporner stores video sources in a JSON blob inside a `<script>` tag:
    /// `{"src":"https://cdn3.eporner.com/...720.mp4","res":720}` etc.
    /// We pick the highest-res source ≤ 720p so files aren't massive.
    /// Fetch the video page HTML and extract the direct CDN `.mp4` URL.
    ///
    /// This uses the reverse-engineered `calc_hash` to make a query to Eporner's
    /// internal JSON endpoint (`/xhr/video/{video_id}`) with matching referer header.
    /// If that fails or is blocked, it falls back to parsing the JSON-LD schema contentUrl.
    pub async fn get_mp4_url(&self, video_id: &str) -> Result<String> {
        // eporner page URLs are: /video-XXXXXXXX/anything/
        let page_url = format!("https://www.eporner.com/video-{}/jav/", video_id);
        let html_res = self
            .http
            .get(&page_url)
            .header("Referer", "https://www.eporner.com/")
            .send()
            .await;

        let html = match html_res {
            Ok(resp) => resp.error_for_status()?.text().await.unwrap_or_default(),
            Err(e) => return Err(anyhow!("Failed to fetch video page HTML: {}", e)),
        };

        // Try Method 1: Fetch via the player's internal XHR endpoint using the hash
        if let Some(hash_cap) = Regex::new(r#"hash\s*[:=]\s*['"]([0-9a-fA-F]{32})['"]"#)
            .unwrap()
            .captures(&html)
        {
            let vid_hash = hash_cap[1].to_string();
            let safe_hash = calc_hash(&vid_hash);

            let xhr_url = format!("https://www.eporner.com/xhr/video/{}", video_id);
            let xhr_res = self
                .http
                .get(&xhr_url)
                .header("Referer", &page_url)
                .query(&[
                    ("hash", safe_hash.as_str()),
                    ("device", "generic"),
                    ("domain", "www.eporner.com"),
                    ("fallback", "false"),
                ])
                .send()
                .await;

            if let Ok(xhr_resp) = xhr_res {
                if let Ok(xhr_data) = xhr_resp.json::<XhrResponse>().await {
                    if let Some(sources) = xhr_data.sources {
                        let mut best: Option<(u32, String)> = None;

                        // Parse formats (mp4) returned by the XHR
                        // Eporner format keys are e.g. "mp4-h264", "mp4-av1", "hls"
                        for (kind, val) in sources {
                            if kind.contains("mp4") {
                                // val is a JSON map of resolutions to URL objects: {"720p": {"src": "..."}, "360p": {"src": "..."}}
                                if let Some(res_map) = val.as_object() {
                                    for (res_str, src_val) in res_map {
                                        if let Some(src_str) = src_val.get("src").and_then(|v| v.as_str()) {
                                            let res_num: u32 = res_str
                                                .replace("p", "")
                                                .parse()
                                                .unwrap_or(0);
                                            // Pick highest quality ≤ 720p for Discord embed playability
                                            if res_num <= 720 {
                                                if best.as_ref().map_or(true, |(r, _)| res_num > *r) {
                                                    best = Some((res_num, src_str.to_string()));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some((_, url)) = best {
                            return Ok(url);
                        }
                    }
                }
            }
        }

        // Try Method 2: Fall back to Schema metadata block (contentUrl)
        if let Some(content_cap) = Regex::new(r#""contentUrl"\s*:\s*"([^"]+\.mp4)""#)
            .unwrap()
            .captures(&html)
        {
            let content_url = content_cap[1].to_string();
            if !content_url.is_empty() {
                return Ok(content_url);
            }
        }

        // Try Method 3: Legacy JSON Regex (just in case they inline it)
        let legacy_re = Regex::new(r#"\{"src":"(https://[^"]+\.mp4)","res":(\d+)"#).unwrap();
        let mut legacy_best: Option<(u32, String)> = None;
        for cap in legacy_re.captures_iter(&html) {
            let src = cap[1].to_string();
            let res: u32 = cap[2].parse().unwrap_or(0);
            if res <= 720 {
                if legacy_best.as_ref().map_or(true, |(r, _)| res > *r) {
                    legacy_best = Some((res, src));
                }
            }
        }

        if let Some((_, url)) = legacy_best {
            return Ok(url);
        }

        Err(anyhow!(
            "No suitable MP4 source found on eporner video page for id={}",
            video_id
        ))
    }

    /// Full pipeline: search → for each result, resolve MP4 URL.
    /// Returns only videos where MP4 extraction succeeded.
    pub async fn fetch_jav_videos(&self, query: &str, count: u32) -> Result<Vec<EpornerVideo>> {
        let entries = self.search(query, count).await?;
        let mut results = Vec::new();

        for entry in entries {
            // Build page URL
            let slug = slug_from_title(&entry.title);
            let page_url = format!("https://www.eporner.com/video-{}/{}/", entry.id, slug);

            // Pick best thumbnail
            let thumb_url = best_thumb(&entry);

            let mp4_url = match self.get_mp4_url(&entry.id).await {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("Could not get MP4 for eporner id={}: {}", entry.id, e);
                    continue;
                }
            };

            results.push(EpornerVideo {
                id: entry.id,
                title: entry.title,
                page_url,
                mp4_url,
                thumb_url,
                duration: entry.length_min.unwrap_or_else(|| entry.length.unwrap_or_default()),
                views: entry.views.unwrap_or_default(),
            });
        }

        Ok(results)
    }
}

/// Convert a title to a URL-friendly slug (alphanumeric + hyphens).
fn slug_from_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(50)
        .collect()
}

/// Pick the largest available thumbnail from the thumbs array.
fn best_thumb(entry: &EpornerVideoEntry) -> String {
    if let Some(thumbs) = &entry.thumbs {
        if let Some(best) = thumbs.iter().max_by_key(|t| t.width.unwrap_or(0)) {
            return best.src.clone();
        }
    }
    String::new()
}

// ─── Base36 Hashing helpers ──────────────────────────────────────────────────

fn encode_base_n(mut num: u64, base: u64) -> String {
    if num == 0 {
        return "0".to_string();
    }
    let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();
    while num > 0 {
        result.push(chars[(num % base) as usize]);
        num /= base;
    }
    result.reverse();
    String::from_utf8(result).unwrap_or_default()
}

fn calc_hash(hash_str: &str) -> String {
    let mut result = String::new();
    for i in 0..4 {
        let start = i * 8;
        let end = start + 8;
        if let Some(chunk) = hash_str.get(start..end) {
            if let Ok(val) = u64::from_str_radix(chunk, 16) {
                result.push_str(&encode_base_n(val, 36));
            }
        }
    }
    result
}

