use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};

use super::models::OkXxxVideo;

const BASE_URL: &str = "https://ok.xxx";

/// Pages to rotate through for a fresh feed each tick.
/// Each page index maps to https://ok.xxx/<n>/ (except 1 → https://ok.xxx/)
const MAX_PAGE: u32 = 5;

pub struct OkXxxClient {
    http: Client,
}

impl OkXxxClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { http })
    }

    /// Fetch video listings from a given page (1 = homepage).
    pub async fn fetch_videos(&self, page: u32) -> Result<Vec<OkXxxVideo>> {
        let url = if page <= 1 {
            format!("{}/", BASE_URL)
        } else {
            format!("{}/{}/", BASE_URL, page)
        };

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
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<OkXxxVideo>> {
        let page = (tick % MAX_PAGE as u64) as u32 + 1;
        self.fetch_videos(page).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML parser
// ─────────────────────────────────────────────────────────────────────────────

fn parse_videos(html: &str) -> Result<Vec<OkXxxVideo>> {
    let document = Html::parse_document(html);

    // Each video card: <div class="... thumb-bl-video ...">
    let card_sel    = Selector::parse("div.thumb-bl-video").unwrap();
    // Anchor with href + title inside the thumb wrapper
    let link_sel    = Selector::parse("div.thumb-video > a").unwrap();
    // Lazy-loaded thumbnail — actual URL is in data-original
    let img_sel     = Selector::parse("div.thumb-video img").unwrap();
    // Meta row: [0]=duration, [1]=date, [2]=views
    let meta_sel    = Selector::parse("ul.video-meta li span").unwrap();

    let mut videos = Vec::new();

    for card in document.select(&card_sel) {
        // ── Link + title ───────────────────────────────────────────────────
        let link_el = card.select(&link_sel).next();
        let href = match link_el.and_then(|a| a.value().attr("href")) {
            Some(h) if !h.is_empty() => h,
            _ => continue,
        };

        let title_raw = link_el
            .and_then(|a| a.value().attr("title"))
            .unwrap_or_default();

        // Strip generic "Watch " prefix inserted by the site
        let title = title_raw
            .trim_start_matches("Watch ")
            .trim()
            .to_string();

        // ── Thumbnail ──────────────────────────────────────────────────────
        let thumbnail = match card
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("data-original"))
        {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => continue, // skip cards without a real thumbnail
        };

        // ── Video ID (from href like /video/758382/) ───────────────────────
        let video_id = href
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
            .next_back()
            .unwrap_or_default()
            .to_string();

        if video_id.is_empty() {
            continue;
        }

        // ── Duration + views ───────────────────────────────────────────────
        let spans: Vec<String> = card
            .select(&meta_sel)
            .map(|s| s.inner_html().trim().to_string())
            .collect();

        let duration = spans.first().cloned().unwrap_or_default();
        let views    = spans.get(2).cloned().unwrap_or_default();

        videos.push(OkXxxVideo {
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
