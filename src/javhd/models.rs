use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct JavhdApiResponse {
    pub template: Vec<JavhdVideoEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavhdVideoEntry {
    pub id: u64,
    pub title: String,
    pub thumbs: HashMap<String, String>,
    pub video: Option<String>,
    #[serde(rename = "videoMedium")]
    pub video_medium: Option<String>,
    pub length: Option<String>,
    pub clicks: Option<u64>,
    #[serde(rename = "studioUrl")]
    pub studio_url: Option<String>,
    #[serde(rename = "isFreeCreatorVideo")]
    pub is_free: Option<bool>,
}

/// Resolved video ready for posting
#[derive(Debug, Clone)]
pub struct JavhdVideo {
    pub id: String,
    pub title: String,
    pub page_url: String,
    pub preview_mp4: String,
    pub thumb_url: String,
    pub duration: String,
    pub views: u64,
}
