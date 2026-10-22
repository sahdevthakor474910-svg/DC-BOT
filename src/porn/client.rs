use anyhow::Result;
use reqwest::Client;

use super::models::{RedTubeResponse, RedTubeVideo};

const REDTUBE_API: &str = "https://api.redtube.com/";

/// Search categories we rotate through every tick.
/// Each search maps to a different vibe — studio, category, or keyword.
pub const PORN_SEARCHES: &[&str] = &[
    "indian viral",
    "desi viral",
    "indian bhabhi",
    "desi bhabhi",
    "indian amateur",
    "desi mms",
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
    pub async fn fetch_videos(&self, search: &str, count: u8, page: u32) -> Result<Vec<RedTubeVideo>> {
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
                ("page", &page.to_string()),
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

    /// Scrape the Redtube embed page or video page to find the direct playable .mp4 / .m3u8 URL.
    pub async fn get_mp4_url(&self, video_id_or_url: &str) -> Result<String> {
        let video_id = crate::web::extract_redtube_id(video_id_or_url)
            .unwrap_or_else(|| video_id_or_url.to_string());

        // Strategy A: Fetch official embed player page (embed.redtube.com/?id=...)
        // This has no age-gate modal, is open to webmasters, and contains the media stream definition.
        let embed_url = format!("https://embed.redtube.com/?id={}", video_id);
        if let Ok(resp) = self.http.get(&embed_url)
            .header("Referer", "https://embed.redtube.com/")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    if let Some(stream) = Self::extract_stream_from_html(&html) {
                        return Ok(stream);
                    }
                }
            }
        }

        // Strategy B: Fetch main video page with age-verification bypass cookies
        let page_url = format!("https://www.redtube.com/{}", video_id);
        if let Ok(resp) = self.http.get(&page_url)
            .header("Referer", "https://www.redtube.com/")
            .header("Cookie", "age_verified=1; RDT_AGE=1; has_visited=1; accessAgeDisclaimerPH=1; platform=pc")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    if let Some(stream) = Self::extract_stream_from_html(&html) {
                        return Ok(stream);
                    }
                }
            }
        }

        anyhow::bail!("No direct stream URL found for Redtube video ID {}", video_id)
    }

    fn extract_stream_from_html(html: &str) -> Option<String> {
        let clean_url = |s: &str| -> String {
            s.replace("&amp;", "&")
                .replace(r"\u0026", "&")
                .replace(r"\/", "/")
                .replace(r"\", "")
        };

        // 1. Scan for mediaDefinitions block and extract direct videoUrl values
        if let Some(pos) = html.find("mediaDefinition") {
            let snippet = &html[pos..std::cmp::min(pos + 16384, html.len())];
            if let Ok(vu_re) = regex::Regex::new(r#"(?:"videoUrl"|videoUrl)\s*[:=]\s*["']([^"']+)["']"#) {
                let mut best_m3u8 = None;
                let mut best_mp4_high = None;
                let mut best_mp4_any = None;
                for cap in vu_re.captures_iter(snippet) {
                    let clean = clean_url(&cap[1]);
                    if clean.starts_with("http") {
                        if clean.contains(".m3u8") {
                            if best_m3u8.is_none() {
                                best_m3u8 = Some(clean);
                            }
                        } else if clean.contains(".mp4") && !clean.contains("trailer") {
                            if clean.contains("720") || clean.contains("480") || clean.contains("1080") {
                                if best_mp4_high.is_none() {
                                    best_mp4_high = Some(clean.clone());
                                }
                            }
                            if best_mp4_any.is_none() {
                                best_mp4_any = Some(clean);
                            }
                        }
                    }
                }
                if let Some(u) = best_mp4_high {
                    return Some(u);
                }
                if let Some(u) = best_mp4_any {
                    return Some(u);
                }
                if let Some(u) = best_m3u8 {
                    return Some(u);
                }
            }
        }

        // 2. Scan entire page for any videoUrl property (quoted or unquoted)
        if let Ok(any_url_re) = regex::Regex::new(r#"(?:"videoUrl"|videoUrl|"video_url"|video_url)\s*[:=]\s*["']([^"']+)["']"#) {
            let mut hls_fallback = None;
            let mut mp4_high = None;
            let mut mp4_any = None;
            for cap in any_url_re.captures_iter(html) {
                let clean = clean_url(&cap[1]);
                if clean.starts_with("http") {
                    if clean.contains(".m3u8") {
                        if hls_fallback.is_none() {
                            hls_fallback = Some(clean);
                        }
                    } else if clean.contains(".mp4") && !clean.contains("trailer") {
                        if clean.contains("720") || clean.contains("480") || clean.contains("1080") {
                            if mp4_high.is_none() {
                                mp4_high = Some(clean.clone());
                            }
                        }
                        if mp4_any.is_none() {
                            mp4_any = Some(clean);
                        }
                    }
                }
            }
            if let Some(u) = mp4_high {
                return Some(u);
            }
            if let Some(u) = mp4_any {
                return Some(u);
            }
            if let Some(u) = hls_fallback {
                return Some(u);
            }
        }

        // 3. Try HTML <source> tag
        if let Ok(source_re) = regex::Regex::new(r#"<source[^>]+src=["']([^"']+\.(?:mp4|m3u8)(?:\?[^"']*)?)["']"#) {
            if let Some(cap) = source_re.captures(html) {
                let clean = clean_url(&cap[1]);
                return Some(clean);
            }
        }

        // 4. Try sources: {...}
        if let Ok(sources_re) = regex::Regex::new(r#"sources\s*:\s*(\{[^}]+\})"#) {
            if let Some(cap) = sources_re.captures(html) {
                let inner = &cap[1];
                if let Ok(mp4_re) = regex::Regex::new(r#""([^"]+\.mp4[^"]*)""#) {
                    if let Some(m) = mp4_re.captures(inner) {
                        return Some(clean_url(&m[1]));
                    }
                }
                if let Ok(m3u8_re) = regex::Regex::new(r#""([^"]+\.m3u8[^"]*)""#) {
                    if let Some(m) = m3u8_re.captures(inner) {
                        return Some(clean_url(&m[1]));
                    }
                }
            }
        }

        // 5. Scan for any MP4 or M3U8 CDN URL
        if let Ok(cdn_re) = regex::Regex::new(r#"(https?:\\?/\\?/[^"'\s]+?\.(?:mp4|m3u8)(?:\?[^"'\s<>]*)?)"#) {
            let mut hls_fallback = None;
            let mut mp4_fallback = None;
            for cap in cdn_re.captures_iter(html) {
                let clean = clean_url(&cap[1]);
                if clean.starts_with("http") && !clean.contains("trailer") {
                    if clean.contains(".mp4") && mp4_fallback.is_none() {
                        mp4_fallback = Some(clean);
                    } else if clean.contains(".m3u8") && hls_fallback.is_none() {
                        hls_fallback = Some(clean);
                    }
                }
            }
            if let Some(u) = mp4_fallback {
                return Some(u);
            }
            if let Some(u) = hls_fallback {
                return Some(u);
            }
        }

        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Beeg.tv scraper (SSR HTML — no API key required)
// ─────────────────────────────────────────────────────────────────────────────

const BEEGTV_BASE: &str = "https://beeg.tv";

/// Scrapes the beeg.tv homepage which is Next.js SSR — all video cards
/// are already in the returned HTML (no JS execution needed).
pub struct BeegTvClient {
    http: Client,
}

const BEEG_CATEGORIES: &[&str] = &[
    "amateur", "anal", "asian", "babe", "big-ass", "big-tits",
    "blowjob", "brunette", "creampie", "cumshot", "hardcore",
    "latina", "lesbian", "milf", "pov", "teen",
];

impl BeegTvClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { http })
    }

    /// Fetch latest videos from beeg.tv homepage or rotating categories.
    /// Returns up to `limit` videos by scraping the SSR HTML.
    pub async fn fetch_latest(&self, limit: usize, page: u32) -> Result<Vec<super::models::BeegTvVideo>> {
        let url = if page == 0 {
            BEEGTV_BASE.to_string()
        } else {
            let cat = BEEG_CATEGORIES[(page as usize) % BEEG_CATEGORIES.len()];
            format!("{}/category/{}", BEEGTV_BASE, cat)
        };
        let html = self.http
            .get(&url)
            .header("Accept", "text/html,application/xhtml+xml")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Self::parse_videos(&html, limit)
    }

    fn parse_videos(html: &str, limit: usize) -> Result<Vec<super::models::BeegTvVideo>> {
        use regex::Regex;

        // Each video card is: href="/slug-NUMBER">...alt="title"...duration span...
        // Pattern captures: (1) full slug path, (2) numeric ID, (3) img alt (real title),
        //                   (4) thumbnail path, (5) duration
        let re = Regex::new(
            r#"href="(/([\w-]+-(\d+)))">.*?<img alt="([^"]+)"[^>]+src="(/uploads/thumbnails/[^"]+)".*?<span[^>]*>(\d+:\d+)</span>"#,
        )?;

        let mut videos = Vec::new();
        for cap in re.captures_iter(html) {
            let slug_path = &cap[1];   // e.g. /under-rapid-dick-fire-127461
            let video_id  = cap[3].to_string(); // e.g. 127461
            let title     = html_unescape(&cap[4]);
            let thumb_rel = &cap[5];   // e.g. /uploads/thumbnails/under-...jpg
            let duration  = cap[6].to_string();

            videos.push(super::models::BeegTvVideo {
                video_id,
                title,
                url:       format!("{}{}", BEEGTV_BASE, slug_path),
                thumbnail: format!("{}{}", BEEGTV_BASE, thumb_rel),
                duration,
            });

            if videos.len() >= limit {
                break;
            }
        }

        Ok(videos)
    }
}

fn html_unescape(s: &str) -> String {
    s.replace("&#x27;", "'")
     .replace("&amp;", "&")
     .replace("&quot;", "\"")
     .replace("&lt;", "<")
     .replace("&gt;", ">")
}
