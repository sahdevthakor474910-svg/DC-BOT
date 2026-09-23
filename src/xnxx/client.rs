use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};

use super::models::XnxxVideo;

const BASE_URL: &str = "https://www.xnxx.com";
const MAX_PAGE: u32 = 5;

pub struct XnxxClient {
    http: Client,
}

impl XnxxClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(20))
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
        parse_videos(&html)
    }

    /// Convenience: fetch a rotating page (tick_index mod MAX_PAGE).
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<XnxxVideo>> {
        let page = (tick % MAX_PAGE as u64) as u32;
        self.fetch_videos(page).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML parser
// ─────────────────────────────────────────────────────────────────────────────

fn parse_videos(html: &str) -> Result<Vec<XnxxVideo>> {
    let document = Html::parse_document(html);

    // Each video card: <div class="thumb-block">
    let card_sel  = Selector::parse("div.thumb-block").unwrap();
    // Link + title inside card
    let link_sel  = Selector::parse("p.metadata > a").unwrap();
    // Thumbnail — xnxx uses data-src for lazy loading
    let img_sel   = Selector::parse("img").unwrap();
    // Metadata spans (duration, views etc.)
    let meta_sel  = Selector::parse("p.metadata").unwrap();

    let mut videos = Vec::new();

    for card in document.select(&card_sel) {
        // ── Link + title ───────────────────────────────────────────────────
        let link_el = card.select(&link_sel).next();
        let href = match link_el.and_then(|a| a.value().attr("href")) {
            Some(h) if !h.is_empty() => h,
            _ => continue,
        };

        let title = if let Some(t) = link_el.and_then(|a| a.value().attr("title")) {
            t.trim().to_string()
        } else {
            link_el.map(|a| a.inner_html().trim().to_string()).unwrap_or_default()
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

        // ── Video ID (from href like /video-abc12345/title) ────────────────
        // Extract the segment right after /video-
        let video_id = href
            .split('/')
            .find_map(|seg| {
                if seg.starts_with("video-") {
                    Some(seg.trim_start_matches("video-").to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // fallback: use last non-empty path segment
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

        videos.push(XnxxVideo {
            video_id,
            title,
            url: format!("{}{}", BASE_URL, href),
            thumbnail,
            duration,
            views,
        });
    }

    Ok(videos)
}
