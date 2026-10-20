use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JavSearchResponse {
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub videos: Vec<JavVideoWrapper>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavVideoWrapper {
    pub video: JavVideo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavVideo {
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
}
