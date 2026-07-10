use serde::Deserialize;

// ── Webmasters API search response ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct EpornerSearchResponse {
    pub videos: Vec<EpornerVideoEntry>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct EpornerVideoEntry {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "rate")]
    pub rate: Option<String>,
    #[serde(rename = "views")]
    pub views: Option<String>,
    #[serde(rename = "length")]
    pub length: Option<String>,
    #[serde(rename = "length_min")]
    pub length_min: Option<String>,
    #[serde(rename = "keywords")]
    pub keywords: Option<String>,
    #[serde(rename = "embed")]
    pub embed: Option<String>,
    #[serde(rename = "url")]
    pub url: Option<String>,
    pub thumbs: Option<Vec<EpornerThumb>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EpornerThumb {
    pub src: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Fully resolved video data after HTML scraping for direct MP4.
#[derive(Debug, Clone)]
pub struct EpornerVideo {
    pub id: String,
    pub title: String,
    pub page_url: String,
    pub mp4_url: String,
    pub thumb_url: String,
    pub duration: String,
    pub views: String,
}
