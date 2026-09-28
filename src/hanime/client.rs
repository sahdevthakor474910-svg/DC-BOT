use anyhow::Result;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::debug;

use super::models::HentaiVideo;

const BASE_URL: &str = "https://hentaigasm.com";

/// Pages to rotate through for variety each tick
const PAGES: &[&str] = &[
    "/",                          // Latest (homepage)
    "/?orderby=views",            // Most viewed all time
    "/page/2/",                   // Latest page 2
    "/genre/uncensored/",         // Uncensored
    "/page/3/",                   // Latest page 3
    "/genre/uncensored/page/2/",  // Uncensored page 2
    "/page/4/",                   // Latest page 4
    "/?orderby=views&paged=2",    // Most viewed page 2
];

pub struct HentaiClient {
    http: Client,
}

impl HentaiClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(20))
            .build()?;
        Ok(Self { http })
    }

    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<HentaiVideo>> {
        let page = PAGES[(tick as usize) % PAGES.len()];
        let url = format!("{}{}", BASE_URL, page);
        self.fetch_page(&url).await
    }

    async fn fetch_page(&self, url: &str) -> Result<Vec<HentaiVideo>> {
        let html = self
            .http
            .get(url)
            .header("Referer", "https://hentaigasm.com/")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let document = Html::parse_document(&html);
        let item_sel = Selector::parse("div.item").unwrap();
        let link_sel = Selector::parse("a.clip-link").unwrap();
        let img_sel = Selector::parse("span.clip img").unwrap();
        let views_sel = Selector::parse("p.stats span.views i.count").unwrap();
        let likes_sel = Selector::parse("p.stats span.dp-post-likes i.count").unwrap();

        let mut videos = Vec::new();

        for item in document.select(&item_sel) {
            let link = match item.select(&link_sel).next() {
                Some(el) => el,
                None => continue,
            };

            let href = link.value().attr("href").unwrap_or_default().to_string();
            let title = link.value().attr("title").unwrap_or_default().to_string();
            let id = link.value().attr("data-id").unwrap_or_default().to_string();

            // Skip ads and invalid entries
            if href.is_empty() || title.is_empty() || id.is_empty() {
                continue;
            }
            // Only keep links to hentaigasm.com (skip ad redirects)
            if !href.contains("hentaigasm.com") {
                continue;
            }

            let thumbnail = item
                .select(&img_sel)
                .next()
                .and_then(|img| img.value().attr("src"))
                .unwrap_or_default()
                .to_string();

            let views = item
                .select(&views_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let likes = item
                .select(&likes_sel)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            videos.push(HentaiVideo {
                id,
                title,
                url: href,
                thumbnail,
                views,
                likes,
                mp4_url: String::new(), // resolved later
            });
        }

        Ok(videos)
    }

    /// Fetch the video page and extract the direct .mp4 URL from JWPlayer config.
    /// Pattern: `jwplayer("player_01").setup({ file: "https://hgasm3.com/Title.mp4", ... })`
    pub async fn get_mp4_url(&self, page_url: &str) -> Result<String> {
        let html = self
            .http
            .get(page_url)
            .header("Referer", "https://hentaigasm.com/")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // Method 1: JWPlayer file: "...mp4" pattern
        let re = Regex::new(r#"file:\s*"(https?://[^"]+\.mp4)""#).unwrap();
        if let Some(cap) = re.captures(&html) {
            return Ok(cap[1].to_string());
        }

        // Method 2: download link href="...mp4"
        let re2 = Regex::new(r#"href="(https?://[^"]+\.mp4)"\s*download"#).unwrap();
        if let Some(cap) = re2.captures(&html) {
            return Ok(cap[1].to_string());
        }

        // Method 3: any .mp4 link on hgasm CDN
        let re3 = Regex::new(r#""(https?://hgasm[^"]+\.mp4)""#).unwrap();
        if let Some(cap) = re3.captures(&html) {
            return Ok(cap[1].to_string());
        }

        anyhow::bail!("No MP4 found on page: {}", page_url)
    }

    /// Fetch listing + resolve MP4 URLs for each video
    pub async fn fetch_with_mp4(&self, tick: u64) -> Result<Vec<HentaiVideo>> {
        let mut videos = self.fetch_for_tick(tick).await?;
        let mut resolved = Vec::new();

        for video in videos.iter_mut() {
            match self.get_mp4_url(&video.url).await {
                Ok(mp4) => {
                    video.mp4_url = mp4;
                    resolved.push(video.clone());
                }
                Err(e) => {
                    debug!("🔮 Hentai: skipping {} — no MP4: {}", video.id, e);
                }
            }
            // Small delay to avoid hammering the server
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        Ok(resolved)
    }
}
