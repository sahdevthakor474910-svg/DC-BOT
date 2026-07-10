use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct RedTubeResponse {
    pub videos: Vec<VideoWrapper>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoWrapper {
    pub video: RedTubeVideo,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RedTubeVideo {
    pub video_id: String,
    pub title: String,
    pub url: String,
    pub default_thumb: String,
    pub duration: String,
    pub views: u64,
    pub rating: String,
    pub publish_date: String,
    pub tags: Vec<VideoTag>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoTag {
    pub tag_name: String,
}
