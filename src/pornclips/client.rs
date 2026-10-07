use anyhow::Result;
use reqwest::Client;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::debug;

use super::models::*;

/// Curated high-traffic straight/female search queries on RedGIFs
const SEARCH_QUERIES: &[&str] = &[
    "creampie",
    "blowjob",
    "doggystyle",
    "cowgirl",
    "riding",
    "deepthroat",
    "titfuck",
    "facial",
    "hardcore",
    "milf",
    "amateur",
    "couple",
    "brunette",
    "blonde",
    "latina",
    "asian",
    "petite",
    "pawg",
    "threesome",
    "lingerie",
    "anal",
    "cumshot",
    "fucking",
    "gangbang",
    "squirt",
];

/// Strict filter to guarantee NO solo males, NO gay porn, NO trans content
fn is_straight_female_content(gif: &RedGifsGif) -> bool {
    // 1. Sexuality check: reject gay/bi/trans
    for sex in &gif.sexuality {
        let s = sex.to_lowercase();
        if s.contains("gay") || s.contains("bisexual") || s.contains("trans") {
            return false;
        }
    }

    // 2. ContentType check: reject Solo Male or Male Only
    if let Some(ct) = &gif.content_type {
        let c = ct.to_lowercase();
        if c.contains("solo male") || c.contains("male only") || c.contains("gay") {
            return false;
        }
    }

    // Combine tags, description, and username for text analysis
    let mut text = gif.tags.join(" ").to_lowercase();
    if let Some(desc) = &gif.description {
        text.push(' ');
        text.push_str(&desc.to_lowercase());
    }

    // 3. Blacklist terms (male solo, gay, trans, femboy, cock ratings, etc.)
    const BLACKLIST: &[&str] = &[
        "solo male",
        "male only",
        "male masturbation",
        "male naked",
        "naked male",
        "gay",
        "homosexual",
        "bisexual",
        "twink",
        "femboy",
        "shemale",
        "trans",
        "transgender",
        "trans woman",
        "trans girl",
        "sissy",
        "crossdresser",
        "cross dressing",
        "ladyboy",
        "girl dick",
        "girldick",
        "bulge",
        "cut cock",
        "small dick",
        "little dick",
        "hairy cock",
        "jerk off",
        "jacking off",
        "dick rating",
        "rate my cock",
        "cock rate",
        "femdom",
        "findom",
        "pegging",
        "chubby male",
    ];

    for &word in BLACKLIST {
        if text.contains(word) {
            return false;
        }
    }

    // 4. If content_type is explicitly "Male + Female", it's verified straight couple sex!
    if let Some(ct) = &gif.content_type {
        if ct.contains("Male + Female") {
            return true;
        }
    }

    // 5. Positive female / heterosexual indicator or search query match:
    if text.is_empty() {
        return true;
    }

    const POSITIVE_FEMALE: &[&str] = &[
        "pussy", "tits", "boobs", "milf", "amateur", "babe", "teen", "ass",
        "blowjob", "creampie", "doggystyle", "cowgirl", "riding", "facial",
        "cumshot", "deepthroat", "titfuck", "threesome", "gangbang", "anal",
        "lingerie", "brunette", "blonde", "redhead", "bbw", "pawg", "ebony",
        "latina", "asian", "pornstar", "petite", "curvy", "oral", "orgasm",
        "swallow", "squirt", "masturbating", "fingering", "strip", "nude",
        "naked", "cute", "hot", "couple", "wife", "step", "fuck", "cum",
    ];

    POSITIVE_FEMALE.iter().any(|&word| text.contains(word))
}

pub struct PornClipsClient {
    http: Client,
    token: Arc<RwLock<String>>,
}

impl PornClipsClient {
    pub async fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            )
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

    /// Search RedGIFs clips with auto-retry on 401 using the working `query` parameter
    pub async fn search(&self, query: &str, count: u32, page: u32) -> Result<RedGifsSearchResponse> {
        let url = format!(
            "https://api.redgifs.com/v2/gifs/search?query={}&count={}&page={}",
            url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>(),
            count,
            page
        );

        // First attempt
        let token = self.token.read().await.clone();
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Token expired, refresh and retry
            self.refresh_token().await?;
            let new_token = self.token.read().await.clone();
            let resp = self
                .http
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

    /// Fetch clips for a given tick, rotating queries and pages with fallback across queries to guarantee clips
    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<RedGifsGif>> {
        let mut results = Vec::new();

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(tick);

        // Changes every 10-minute block (600s). Never resets to 0 across restarts!
        let time_block = now_secs / 600;

        let base_idx = ((time_block + tick) as usize) % SEARCH_QUERIES.len();
        let base_page = (((time_block + tick) as u32) % 40) + 1;

        // Try up to 4 queries across pages to guarantee plenty of fresh clips
        for offset in 0..4 {
            let query_idx = (base_idx + offset) % SEARCH_QUERIES.len();
            let query = SEARCH_QUERIES[query_idx];
            let page = ((base_page + offset as u32) % 40) + 1;

            if let Ok(resp) = self.search(query, 80, page).await {
                for g in resp.gifs {
                    let has_sd = g.urls.as_ref().and_then(|u| u.sd.as_ref()).is_some();
                    let dur = g.duration.unwrap_or(15.0);
                    if has_sd && dur <= 60.0 && dur >= 2.0 && is_straight_female_content(&g) {
                        results.push(g);
                    }
                }
            }

            if results.len() >= 20 {
                break;
            }
        }

        debug!("🎬 PornClips: collected {} valid clips for tick {}", results.len(), tick);
        Ok(results)
    }

    /// Download raw MP4 bytes from a URL
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }
}
