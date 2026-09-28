use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HanimeApiResponse {
    pub data: Vec<HanimeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HanimeEntry {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub views: u64,
    pub cover_url: Option<String>,
    pub poster_url: Option<String>,
    pub brand: Option<String>,
    pub likes: Option<u64>,
    pub dislikes: Option<u64>,
    pub tags: Option<Vec<String>>,
    pub created_at_unix: Option<i64>,
    pub released_at_unix: Option<i64>,
}

/// Resolved hentai video ready for posting
#[derive(Debug, Clone)]
pub struct HanimeVideo {
    pub id: String,
    pub title: String,
    pub page_url: String,
    pub cover_url: String,
    pub brand: String,
    pub views: u64,
    pub likes: u64,
    pub tags: Vec<String>,
}
