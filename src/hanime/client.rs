use anyhow::Result;
use reqwest::Client;
use super::models::{HanimeApiResponse, HanimeVideo};

const API_URL: &str = "https://guest.freeanimehentai.net/api/v11/search_hvs";

/// Rotation configs: (order_by, ordering, page)
const API_CONFIGS: &[(&str, &str, u32)] = &[
    ("trending",          "desc", 0),
    ("likes",             "desc", 0),
    ("views",             "desc", 0),
    ("created_at_unix",   "desc", 0),
    ("trending",          "desc", 1),
    ("likes",             "desc", 1),
];

pub struct HanimeClient {
    http: Client,
}

impl HanimeClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { http })
    }

    pub async fn fetch_for_tick(&self, tick: u64) -> Result<Vec<HanimeVideo>> {
        let idx = (tick as usize) % API_CONFIGS.len();
        let (order_by, ordering, page) = API_CONFIGS[idx];
        self.fetch_videos(order_by, ordering, page).await
    }

    async fn fetch_videos(&self, order_by: &str, ordering: &str, page: u32) -> Result<Vec<HanimeVideo>> {
        let resp = self.http
            .get(API_URL)
            .query(&[
                ("order_by", order_by),
                ("ordering", ordering),
                ("page", &page.to_string()),
                ("count", "10"),
            ])
            .header("Referer", "https://hanime.tv/")
            .send()
            .await?
            .error_for_status()?
            .json::<HanimeApiResponse>()
            .await?;

        let videos: Vec<HanimeVideo> = resp.data
            .into_iter()
            .map(|entry| {
                let page_url = format!("https://hanime.tv/videos/hentai/{}", entry.slug);
                let cover = entry.cover_url
                    .or(entry.poster_url)
                    .unwrap_or_default();

                HanimeVideo {
                    id: entry.id.to_string(),
                    title: entry.name,
                    page_url,
                    cover_url: cover,
                    brand: entry.brand.unwrap_or_else(|| "Unknown".to_string()),
                    views: entry.views,
                    likes: entry.likes.unwrap_or(0),
                    tags: entry.tags.unwrap_or_default(),
                }
            })
            .collect();

        Ok(videos)
    }
}
