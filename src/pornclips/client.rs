use anyhow::Result;
use reqwest::Client;
use tokio::sync::RwLock;
use std::sync::Arc;

use super::models::*;

/// Search queries to rotate through each tick for variety
const SEARCH_QUERIES: &[&str] = &[
    "trending",
    "hot",
    "amateur",
    "blowjob",
    "teen",
    "milf",
    "riding",
    "threesome",
    "anal",
    "creampie",
    "pawg",
    "bbc",
];

pub struct PornClipsClient {
    http: Client,
    token: Arc<RwLock<String>>,
}

impl PornClipsClient {
    pub async fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        
        let token = Self::fetch_token_static(&http).await?;
        Ok(Self {
            http,
            token: Arc::new(RwLock::new(token)),
        })
    }

    async fn fetch_token_static(http: &Client) -> Result<String> {
        let resp: RedGifsAuthResponse = http
            .get("https://api.redgifs.com/v2/auth/temporary")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.token)
    }

    async fn refresh_token(&self) -> Result<()> {
        let new_token = Self::fetch_token_static(&self.http).await?;
        let mut t = self.token.write().await;
        *t = new_token;
        Ok(())
    }

    /// Search RedGIFs with auto-retry on 401 (token expired)
    pub async fn search(&self, query: &str, count: u32, page: u32) -> Result<RedGifsSearchResponse> {
        let url = format!(
            "https://api.redgifs.com/v2/gifs/search?search_text={}&count={}&page={}",
            url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>(),
            count,
            page
        );

        // First attempt
        let token = self.token.read().await.clone();
        let resp = self.http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Token expired, refresh and retry
            self.refresh_token().await?;
            let new_token = self.token.read().await.clone();
            let resp = self.http
                .get(&url)
                .header("Authorization", format!("Bearer {}", new_token))
                .send()
                .await?
                .error_for_status()?;
            return Ok(resp.json().await?);
        }

        let resp = resp.error_for_status()?;
        Ok(resp.json().await?)
    }

    /// Fetch clips for a given tick, rotating queries and pages
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<RedGifsGif>> {
        let query_idx = (tick as usize) % SEARCH_QUERIES.len();
        let query = SEARCH_QUERIES[query_idx];
        let page = ((tick as u32) / (SEARCH_QUERIES.len() as u32)) % 5 + 1;

        let resp = self.search(query, 10, page).await?;
        
        // Filter: only keep clips with SD URL and duration under 120 seconds
        let clips: Vec<RedGifsGif> = resp.gifs
            .into_iter()
            .filter(|g| g.urls.sd.is_some() && g.duration <= 120.0 && g.duration > 3.0)
            .collect();

        Ok(clips)
    }
}
