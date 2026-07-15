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
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub rating: String,
    pub publish_date: String,
    pub tags: Vec<VideoTag>,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum StringOrNum {
        String(String),
        Float(f64),
        Int(i64),
    }

    match StringOrNum::deserialize(deserializer)? {
        StringOrNum::String(s) => Ok(s),
        StringOrNum::Float(f) => Ok(f.to_string()),
        StringOrNum::Int(i) => Ok(i.to_string()),
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoTag {
    pub tag_name: String,
}
