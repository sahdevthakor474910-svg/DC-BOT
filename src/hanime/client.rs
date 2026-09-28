use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;

use super::models::{EpornerHentaiEntry, EpornerHentaiSearchResponse, HanimeVideo};

#[derive(Deserialize, Debug)]
struct XhrResponse {
    sources: Option<std::collections::HashMap<String, serde_json::Value>>,
}

const EPORNER_API: &str = "https://www.eporner.com/api/v2/video/search/";

/// Curated Hentai and Anime queries rotated each tick for varied high-quality content.
pub const HENTAI_SEARCHES: &[&str] = &[
    "hentai uncensored",
    "hentai anime uncensored",
    "anime hentai",
    "3d hentai",
    "hentai animation",
    "sfm hentai",
    "hentai sub",
    "overwatch 3d hentai",
    "hentai full",
    "genshin hentai",
];

pub struct HanimeClient {
    http: Client,
}

impl HanimeClient {
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

    /// Search videos using Eporner's Webmasters API with hentai queries.
    pub async fn search(&self, query: &str, count: u32) -> Result<Vec<EpornerHentaiEntry>> {
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
            .json::<EpornerHentaiSearchResponse>()
            .await?;

        Ok(resp.videos)
    }

    /// Fetch the video page HTML and extract the direct CDN `.mp4` URL.
    pub async fn get_mp4_url(&self, video_id: &str) -> Result<String> {
        let page_url = format!("https://www.eporner.com/video-{}/hentai/", video_id);
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

                        for (kind, val) in sources {
                            if kind.contains("mp4") {
                                if let Some(res_map) = val.as_object() {
                                    for (res_str, src_val) in res_map {
                                        if let Some(src_str) = src_val.get("src").and_then(|v| v.as_str()) {
                                            let res_num: u32 = res_str
                                                .replace("p", "")
                                                .parse()
                                                .unwrap_or(0);
                                            // Pick highest quality <= 720p for Discord embed playback
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

        // Try Method 3: Legacy JSON Regex
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
            "No suitable MP4 source found on video page for id={}",
            video_id
        ))
    }

    /// Fetch videos for a given tick, resolving direct MP4 links
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<HanimeVideo>> {
        let query = HENTAI_SEARCHES[(tick as usize) % HENTAI_SEARCHES.len()];
        let entries = self.search(query, 8).await?;
        let mut results = Vec::new();

        for entry in entries {
            let slug = slug_from_title(&entry.title);
            let page_url = format!("https://www.eporner.com/video-{}/{}/", entry.id, slug);

            let thumb = entry
                .thumbs
                .as_ref()
                .and_then(|thumbs| thumbs.iter().max_by_key(|t| t.width.unwrap_or(0)))
                .map(|t| t.src.clone())
                .or_else(|| entry.default_thumb.map(|t| t.src))
                .unwrap_or_default();

            let mp4_url = match self.get_mp4_url(&entry.id).await {
                Ok(url) => url,
                Err(e) => {
                    tracing::debug!("Could not resolve MP4 for hentai id={}: {}", entry.id, e);
                    continue;
                }
            };

            results.push(HanimeVideo {
                id: entry.id,
                title: entry.title,
                page_url,
                mp4_url,
                cover_url: thumb,
                duration: entry.length_min.unwrap_or_else(|| entry.length.unwrap_or_else(|| "?".to_string())),
                views: entry.views.unwrap_or_else(|| "0".to_string()),
            });
        }

        Ok(results)
    }
}

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
