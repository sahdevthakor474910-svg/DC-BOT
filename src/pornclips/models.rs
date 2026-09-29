use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsAuthResponse {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsSearchResponse {
    pub gifs: Vec<RedGifsGif>,
    pub page: u32,
    pub pages: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsGif {
    pub id: String,
    pub duration: f64,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "hasAudio")]
    pub has_audio: bool,
    pub height: u32,
    pub width: u32,
    pub likes: u64,
    pub views: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    pub urls: RedGifsUrls,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsUrls {
    #[serde(default)]
    pub hd: Option<String>,
    #[serde(default)]
    pub sd: Option<String>,
    #[serde(default)]
    pub poster: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
}
