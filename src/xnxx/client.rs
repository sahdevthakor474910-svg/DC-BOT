use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};

use super::models::XnxxVideo;

const BASE_URL: &str = "https://www.xnxx.com";
const MAX_PAGE: u32 = 15;

pub struct XnxxClient {
    http: Client,
}

impl XnxxClient {
    pub fn new() -> Result<Self> {
        Self::new_with_timeout(20)
    }

    pub fn new_with_timeout(timeout_secs: u64) -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()?;
        Ok(Self { http })
    }

    /// Fetch video listings from a given page (0-indexed).
    /// URL pattern: https://www.xnxx.com/best/week/{page}
    pub async fn fetch_videos(&self, page: u32) -> Result<Vec<XnxxVideo>> {
        let url = format!("{}/best/week/{}", BASE_URL, page);
        let html = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_videos(&html, BASE_URL)
    }

    /// Convenience: fetch a rotating page (tick_index mod MAX_PAGE).
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<XnxxVideo>> {
        let page = (tick % MAX_PAGE as u64) as u32;
        self.fetch_videos(page).await
    }

    /// Search videos on XNXX by keyword (e.g. "indian mms").
    /// URL pattern: https://www.xnxx.com/search/{query} or https://www.xnxx.com/search/{query}/{page}
    pub async fn search_videos(&self, query: &str, page: u32) -> Result<Vec<XnxxVideo>> {
        let clean_query = query.trim().replace(' ', "+");
        let url = if page <= 1 {
            format!("{}/search/{}", BASE_URL, clean_query)
        } else {
            format!("{}/search/{}/{}", BASE_URL, clean_query, page)
        };
        let html = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_videos(&html, BASE_URL)
    }

    /// Search videos on XVideos by keyword (e.g. "indian mms", "college girl mms").
    /// URL pattern: https://www.xvideos.com/?k={query}&p={page}
    pub async fn search_xvideos(&self, query: &str, page: u32) -> Result<Vec<XnxxVideo>> {
        let clean_query = query.trim().replace(' ', "+");
        let url = format!("https://www.xvideos.com/?k={}&p={}", clean_query, page);
        let html = self
            .http
            .get(&url)
            .header("Referer", "https://www.xvideos.com/")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_videos(&html, "https://www.xvideos.com")
    }

    /// Fetch video page HTML and extract direct MP4 or video source URL.
    pub async fn get_mp4_url(&self, video_page_url: &str) -> Result<String> {
        let referer = if video_page_url.contains("xvideos.com") {
            "https://www.xvideos.com/"
        } else {
            "https://www.xnxx.com/"
        };
        let html = self
            .http
            .get(video_page_url)
            .header("Referer", referer)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // 1. html5player.setVideoUrlHigh('https://...mp4')
        let re_high = regex::Regex::new(r#"html5player\.setVideoUrlHigh\(['"]([^'"]+)['"]\)"#)?;
        if let Some(cap) = re_high.captures(&html) {
            let u = &cap[1];
            if !u.is_empty() {
                return Ok(u.trim_end_matches('/').to_string());
            }
        }

        // 2. html5player.setVideoUrlLow('https://...mp4')
        let re_low = regex::Regex::new(r#"html5player\.setVideoUrlLow\(['"]([^'"]+)['"]\)"#)?;
        if let Some(cap) = re_low.captures(&html) {
            let u = &cap[1];
            if !u.is_empty() {
                return Ok(u.trim_end_matches('/').to_string());
            }
        }

        // 3. html5player.setVideoHLS('https://...m3u8')
        let re_hls = regex::Regex::new(r#"html5player\.setVideoHLS\(['"]([^'"]+)['"]\)"#)?;
        if let Some(cap) = re_hls.captures(&html) {
            let u = &cap[1];
            if !u.is_empty() {
                return Ok(u.trim_end_matches('/').to_string());
            }
        }

        // 4. General setVideoUrl('https://...')
        let re_gen = regex::Regex::new(r#"html5player\.setVideoUrl\(['"]([^'"]+)['"]\)"#)?;
        if let Some(cap) = re_gen.captures(&html) {
            let u = &cap[1];
            if !u.is_empty() {
                return Ok(u.trim_end_matches('/').to_string());
            }
        }

        // 5. Broad regex fallback for any CDN stream inside script
        let re_broad = regex::Regex::new(r#"https?://[^"'\s<>]+\.(?:mp4|m3u8)[^"'\s<>]*"#)?;
        for mat in re_broad.find_iter(&html) {
            let s = mat.as_str();
            if !s.contains("trailer") && !s.contains("preview") && !s.contains("thumb") {
                return Ok(s.trim_end_matches('/').to_string());
            }
        }

        anyhow::bail!("No direct MP4/stream URL found on video page: {}", video_page_url)
    }

    /// Fetch low-resolution direct MP4 (ideal for Discord < 8MB video attachment).
    pub async fn get_low_mp4_url(&self, video_page_url: &str) -> Result<String> {
        let referer = if video_page_url.contains("xvideos.com") {
            "https://www.xvideos.com/"
        } else {
            "https://www.xnxx.com/"
        };
        let html = self
            .http
            .get(video_page_url)
            .header("Referer", referer)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // 1. html5player.setVideoUrlLow('https://...mp4')
        let re_low = regex::Regex::new(r#"html5player\.setVideoUrlLow\(['"]([^'"]+)['"]\)"#)?;
        if let Some(cap) = re_low.captures(&html) {
            let u = &cap[1];
            if !u.is_empty() {
                return Ok(u.trim_end_matches('/').to_string());
            }
        }

        // 2. html5player.setVideoUrlHigh('https://...mp4')
        let re_high = regex::Regex::new(r#"html5player\.setVideoUrlHigh\(['"]([^'"]+)['"]\)"#)?;
        if let Some(cap) = re_high.captures(&html) {
            let u = &cap[1];
            if !u.is_empty() {
                return Ok(u.trim_end_matches('/').to_string());
            }
        }

        self.get_mp4_url(video_page_url).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML parser
// ─────────────────────────────────────────────────────────────────────────────

fn parse_videos(html: &str, base_url: &str) -> Result<Vec<XnxxVideo>> {
    let document = Html::parse_document(html);

    // Each video card: <div class="thumb-block">
    let card_sel = Selector::parse("div.thumb-block").unwrap();
    // Link + title inside card: <p class="title"><a href="..." title="..."> or <div class="thumb"><a href="...">
    let link_sel = Selector::parse("p.title a, div.thumb-under a, div.thumb a, a[href*='/video-']").unwrap();
    // Thumbnail — xnxx uses data-src for lazy loading
    let img_sel  = Selector::parse("img").unwrap();
    // Metadata spans (duration, views etc.)
    let meta_sel = Selector::parse("p.metadata").unwrap();

    let mut videos = Vec::new();

    for card in document.select(&card_sel) {
        // ── Link + title ───────────────────────────────────────────────────
        let link_el = card.select(&link_sel).find(|a| {
            a.value().attr("href").map_or(false, |h| h.contains("/video-") || h.contains("/video"))
        });
        let href = match link_el.and_then(|a| a.value().attr("href")) {
            Some(h) if !h.is_empty() => h,
            _ => continue,
        };

        let title = if let Some(t) = link_el.and_then(|a| a.value().attr("title")) {
            t.trim().to_string()
        } else {
            link_el.map(|a| a.text().collect::<Vec<_>>().join(" ").trim().to_string()).unwrap_or_default()
        };

        if title.is_empty() {
            continue;
        }

        // ── Thumbnail ──────────────────────────────────────────────────────
        let thumbnail = card
            .select(&img_sel)
            .next()
            .and_then(|img| {
                img.value().attr("data-src")
                    .or_else(|| img.value().attr("data-original"))
                    .or_else(|| img.value().attr("src"))
            })
            .unwrap_or("")
            .to_string();

        if thumbnail.is_empty() || thumbnail.contains("data:image") {
            continue;
        }

        // ── Video ID (from href like /video-abc12345/title or /video12345/title) ───
        let video_id = href
            .split('/')
            .find_map(|seg| {
                if seg.starts_with("video-") {
                    Some(seg.trim_start_matches("video-").to_string())
                } else if seg.starts_with("video") && seg.len() > 5 {
                    Some(seg.trim_start_matches("video").to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                href.trim_matches('/')
                    .split('/')
                    .last()
                    .unwrap_or_default()
                    .to_string()
            });

        if video_id.is_empty() {
            continue;
        }

        // ── Duration + views from metadata paragraph ───────────────────────
        let mut duration = String::new();
        let mut views = String::new();

        for meta_p in card.select(&meta_sel) {
            let text = meta_p.text().collect::<Vec<_>>().join(" ");
            let text = text.trim().to_string();
            // Duration usually looks like "12:34" or "1:23:45"
            if duration.is_empty() {
                if let Some(part) = text.split_whitespace().find(|s| {
                    s.contains(':') && s.chars().all(|c| c.is_ascii_digit() || c == ':')
                }) {
                    duration = part.to_string();
                }
            }
            // Views usually ends with K, M, or B
            if views.is_empty() {
                if let Some(part) = text.split_whitespace().find(|s| {
                    let last = s.chars().last().unwrap_or(' ');
                    (last == 'K' || last == 'M' || last == 'B')
                        && s[..s.len()-1].chars().all(|c| c.is_ascii_digit() || c == '.')
                }) {
                    views = part.to_string();
                }
            }
        }

        let full_url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("{}{}", base_url, href)
        };

        videos.push(XnxxVideo {
            video_id,
            title,
            url: full_url,
            thumbnail,
            duration,
            views,
        });
    }

    if videos.is_empty() {
        // Fallback regex matching standard search cards:
        if let Ok(re) = regex::Regex::new(r#"<p[^>]*class="title"[^>]*>\s*<a\s+href="([^"]+)"(?:\s+title="([^"]+)")?[^>]*>(?:([^<]+)</a>)?"#) {
            for cap in re.captures_iter(html) {
                let href = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let title = cap.get(2).or_else(|| cap.get(3)).map(|m| m.as_str()).unwrap_or("").trim();
                if href.is_empty() || title.is_empty() {
                    continue;
                }
                let video_id = href
                    .split('/')
                    .find_map(|seg| {
                        if seg.starts_with("video-") {
                            Some(seg.trim_start_matches("video-").to_string())
                        } else if seg.starts_with("video") && seg.len() > 5 {
                            Some(seg.trim_start_matches("video").to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| href.trim_matches('/').split('/').last().unwrap_or_default().to_string());
                if !video_id.is_empty() {
                    let full_url = if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("{}{}", base_url, href)
                    };
                    videos.push(XnxxVideo {
                        video_id,
                        title: title.to_string(),
                        url: full_url,
                        thumbnail: String::new(),
                        duration: String::new(),
                        views: String::new(),
                    });
                }
            }
        }
    }

    Ok(videos)
}
