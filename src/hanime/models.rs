use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct EpornerHentaiSearchResponse {
    pub videos: Vec<EpornerHentaiEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EpornerHentaiEntry {
    #[serde(rename = "id", deserialize_with = "deserialize_string_or_number")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub views: Option<String>,
    #[serde(default)]
    pub length_min: Option<String>,
    #[serde(default)]
    pub length: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    pub thumbs: Option<Vec<EpornerHentaiThumb>>,
    #[serde(default)]
    pub default_thumb: Option<EpornerHentaiThumb>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EpornerHentaiThumb {
    pub src: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Resolved hentai video ready for posting
#[derive(Debug, Clone)]
pub struct HanimeVideo {
    pub id: String,
    pub title: String,
    pub page_url: String,
    pub mp4_url: String,
    pub cover_url: String,
    pub duration: String,
    pub views: String,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnyVal {
        String(String),
        Number(serde_json::Number),
    }

    match AnyVal::deserialize(deserializer)? {
        AnyVal::String(s) => Ok(s),
        AnyVal::Number(n) => Ok(n.to_string()),
    }
}
