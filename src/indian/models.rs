use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IndianSearchResponse {
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub videos: Vec<IndianVideoWrapper>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndianVideoWrapper {
    pub video: IndianVideo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndianVideo {
    #[serde(rename = "video_id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub views: serde_json::Value,
    #[serde(default)]
    pub default_thumb: String,
    pub url: String,
    #[serde(default)]
    pub direct_mp4: Option<String>,
}
