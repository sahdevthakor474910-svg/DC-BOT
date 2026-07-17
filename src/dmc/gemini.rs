use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Extracted data from a DMC: Peak of Combat boss battle screenshot.
#[derive(Debug, Deserialize)]
pub struct BossResult {
    pub boss_name: String,
    pub dmg_pts: i64,
    pub boss_pts: i64,
    pub has_bonus: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Gemini API request / response shapes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<Content<'a>>,
}

#[derive(Serialize)]
struct Content<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part<'a> {
    Text { text: &'a str },
    InlineData { inline_data: InlineData },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String, // base64
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Deserialize, Debug)]
struct ContentResponse {
    parts: Vec<TextPart>,
}

#[derive(Deserialize, Debug)]
struct TextPart {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GeminiError {
    message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Prompt
// ─────────────────────────────────────────────────────────────────────────────

const ANALYSIS_PROMPT: &str = r#"You are analyzing a Devil May Cry: Peak of Combat boss battle results screenshot. Extract EXACTLY these values:
1. Boss name
2. DMG PTS (large number next to DMG PTS:)
3. Boss PTS (large number next to Boss PTS)
4. Has X120% Bonus? (yes or no)

Reply ONLY in this exact JSON format:
{
  "boss_name": "Devil Mite",
  "dmg_pts": 1022497809,
  "boss_pts": 1033793224,
  "has_bonus": false
}
Numbers must be plain integers, no commas.
has_bonus is true only if X120% appears on screen."#;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Download `image_url`, encode it to base64, send it to Gemini 1.5 Flash,
/// and return the extracted `BossResult`.
pub async fn analyze_screenshot(
    http: &reqwest::Client,
    api_key: &str,
    image_url: &str,
) -> Result<BossResult> {
    // 1. Download the image bytes
    let img_bytes = http
        .get(image_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    // 2. Detect MIME type from magic bytes
    let mime = detect_mime(&img_bytes);

    // 3. Base64-encode
    let b64 = general_purpose::STANDARD.encode(&img_bytes);

    // 4. Build Gemini request
    let body = GeminiRequest {
        contents: vec![Content {
            parts: vec![
                Part::InlineData {
                    inline_data: InlineData {
                        mime_type: mime.to_string(),
                        data: b64,
                    },
                },
                Part::Text {
                    text: ANALYSIS_PROMPT,
                },
            ],
        }],
    };

    // 5. Call Gemini API
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent?key={}",
        api_key
    );

    let resp: GeminiResponse = http
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // 6. Handle API-level errors
    if let Some(err) = resp.error {
        return Err(anyhow!("Gemini API error: {}", err.message));
    }

    // 7. Extract the raw text from the first candidate
    let raw_text = resp
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .ok_or_else(|| anyhow!("No content returned from Gemini"))?;

    // 8. Strip potential markdown code fences and parse JSON
    let json_str = strip_json_fences(&raw_text);
    let result: BossResult = serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Failed to parse Gemini JSON ({}): {}", e, raw_text))?;

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn detect_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "image/jpeg" // safe default
    }
}

/// Remove ```json ... ``` or ``` ... ``` wrappers if Gemini wraps its output.
fn strip_json_fences(s: &str) -> &str {
    let s = s.trim();
    // Try stripping ```json\n...\n```
    if let Some(inner) = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
    {
        if let Some(cleaned) = inner.strip_suffix("```") {
            return cleaned.trim();
        }
    }
    s
}
