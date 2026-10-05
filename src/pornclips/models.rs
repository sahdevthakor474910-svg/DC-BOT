use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsAuthResponse {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsSearchResponse {
    #[serde(default)]
    pub gifs: Vec<RedGifsGif>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub pages: Option<u32>,
    #[serde(default)]
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedGifsGif {
    pub id: String,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "hasAudio", default)]
    pub has_audio: Option<bool>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub likes: Option<u64>,
    #[serde(default)]
    pub views: Option<u64>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "userName", default)]
    pub user_name: Option<String>,
    #[serde(rename = "contentType", default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub sexuality: Vec<String>,
    #[serde(default)]
    pub urls: Option<RedGifsUrls>,
}

#[derive(Debug, Clone, Deserialize, Default)]
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
