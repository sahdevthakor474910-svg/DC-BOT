use serde::{Deserialize, Deserializer};

// ── Webmasters API search response ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct EpornerSearchResponse {
    pub videos: Vec<EpornerVideoEntry>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct EpornerVideoEntry {
    #[serde(rename = "id", deserialize_with = "deserialize_string_or_number")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "rate", deserialize_with = "deserialize_string_or_number_opt", default)]
    pub rate: Option<String>,
    #[serde(rename = "views", deserialize_with = "deserialize_string_or_number_opt", default)]
    pub views: Option<String>,
    #[serde(rename = "length", deserialize_with = "deserialize_string_or_number_opt", default)]
    pub length: Option<String>,
    #[serde(rename = "length_min", deserialize_with = "deserialize_string_or_number_opt", default)]
    pub length_min: Option<String>,
    #[serde(rename = "keywords", default)]
    pub keywords: Option<String>,
    #[serde(rename = "embed", default)]
    pub embed: Option<String>,
    #[serde(rename = "url", default)]
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

// ── Custom Deserializers ─────────────────────────────────────────────────────

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

fn deserialize_string_or_number_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnyVal {
        String(String),
        Number(serde_json::Number),
        Null,
    }

    match Option::<AnyVal>::deserialize(deserializer)? {
        Some(AnyVal::String(s)) => Ok(Some(s)),
        Some(AnyVal::Number(n)) => Ok(Some(n.to_string())),
        Some(AnyVal::Null) | None => Ok(None),
    }
}
