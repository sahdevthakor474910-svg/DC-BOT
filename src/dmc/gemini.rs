use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Public types – what callers receive
// ─────────────────────────────────────────────────────────────────────────────

/// A single player entry extracted from a leaderboard screenshot.
#[derive(Debug, Deserialize)]
pub struct LeaderboardPlayer {
    pub rank: u32,
    pub name: String,
    pub total_pts: i64,
}

/// The two kinds of screenshots the bot can receive.
#[derive(Debug)]
pub enum ScreenshotData {
    /// Post-battle results screen (shows DMG PTS / Boss PTS).
    Results {
        boss_name: String,
        dmg_pts: i64,
        boss_pts: i64,
        has_bonus: bool,
    },
    /// Leaderboard / ranking screen.
    Leaderboard {
        boss_name: String,
        has_bonus: bool,
        players: Vec<LeaderboardPlayer>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw Gemini JSON shapes (intermediate deserialization)
// ─────────────────────────────────────────────────────────────────────────────

/// Gemini can return either screen type; we deserialise via a "type" tag.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawScreenshot {
    Results {
        boss_name: String,
        dmg_pts: i64,
        boss_pts: i64,
        has_bonus: bool,
    },
    Leaderboard {
        boss_name: String,
        has_bonus: bool,
        players: Vec<LeaderboardPlayer>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Gemini API request / response wire shapes
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

const ANALYSIS_PROMPT: &str = r#"You are analyzing Devil May Cry: Peak of Combat screenshots.

First identify the screenshot type:
- "results"     = shows DMG PTS, Reward PTS, Boss PTS after a battle
- "leaderboard" = shows Ranking, Player Name, Total PTS

═══════════════════════════════════
RESULTS SCREENSHOT
═══════════════════════════════════
Extract:
1. Boss name
2. DMG PTS (large number next to "DMG PTS:")
3. Boss PTS (large number next to "Boss PTS")
4. Has X120% bonus? (true/false)

Reply in this JSON:
{
  "type": "results",
  "boss_name": "Devil Mite",
  "dmg_pts": 1022497809,
  "boss_pts": 1033793224,
  "has_bonus": false
}

═══════════════════════════════════
LEADERBOARD SCREENSHOT
═══════════════════════════════════
Extract:
1. Boss name (shown on left side tab that is highlighted/selected)
2. Has X120% bonus? (true/false - check if this boss has bonus)
3. All visible players with rank, name, total pts

Reply in this JSON:
{
  "type": "leaderboard",
  "boss_name": "Calibur",
  "has_bonus": false,
  "players": [
    {"rank": 1, "name": "中國台灣省", "total_pts": 1033499653},
    {"rank": 2, "name": "KèLiêuMạng.VN", "total_pts": 1033179794},
    {"rank": 3, "name": "Desuwyy!", "total_pts": 1032576203},
    {"rank": 4, "name": "★PinjamDulu`Seratus★", "total_pts": 1030632084}
  ]
}

═══════════════════════════════════
RULES FOR BOTH
═══════════════════════════════════
- Numbers must be plain integers, no commas
- has_bonus is true only if boss is in bonus list below
- Extract ALL visible players in leaderboard
- If value unclear, use 0

BONUS BOSSES (X120%):
Hell Shade, Beowulf, Plutone, Vergil, Dante, Hell Commander, Hell·Commander"#;

// ─────────────────────────────────────────────────────────────────────────────
// gemini-3.1-flash-lite is the modern free-tier model (15 RPM / 1500 RPD) that supports vision.
// gemini-3.5-flash is our fallback model (10 RPM / 250 RPD).
// ─────────────────────────────────────────────────────────────────────────────
const GEMINI_MODELS: &[&str] = &[
    "gemini-3.1-flash-lite",
    "gemini-3.5-flash",
];

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Download `image_url`, encode it as base64, send it to Gemini, and return
/// the extracted [`ScreenshotData`].
pub async fn analyze_screenshot(
    http: &reqwest::Client,
    api_key: &str,
    image_url: &str,
) -> Result<ScreenshotData> {
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

    // 5. Try each model in order; retry up to 2× on 429 with backoff
    let mut last_err = anyhow::anyhow!("No Gemini models available");
    'models: for model in GEMINI_MODELS {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );
        for attempt in 0u32..3 {
            if attempt > 0 {
                // Exponential backoff: 5s, 15s
                let wait = std::time::Duration::from_secs(5 * (3u64.pow(attempt - 1)));
                tracing::warn!("Gemini 429 on {model}, waiting {wait:?} before retry {attempt}/2…");
                tokio::time::sleep(wait).await;
            }
            let http_resp = http.post(&url).json(&body).send().await?;
            let status = http_resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_err = anyhow!("Rate-limited on model {}", model);
                continue; // retry same model
            }
            if !status.is_success() {
                let err_text = http_resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                last_err = anyhow!("HTTP error {} on model {}: {}", status, model, err_text);
                continue 'models; // try next model
            }

            let resp: GeminiResponse = match http_resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = anyhow!("JSON decode failed on model {}: {}", model, e);
                    continue 'models; // try next model
                }
            };

            if let Some(err) = resp.error {
                last_err = anyhow!("Gemini API error on {}: {}", model, err.message);
                continue 'models; // try next model
            }

            // ── success path ──────────────────────────────────────────────────
            let raw_text = resp
                .candidates
                .and_then(|c| c.into_iter().next())
                .and_then(|c| c.content.parts.into_iter().next())
                .map(|p| p.text)
                .ok_or_else(|| anyhow!("No content from Gemini model {}", model))?;

            let json_str = strip_json_fences(&raw_text);
            let raw: RawScreenshot = serde_json::from_str(json_str)
                .map_err(|e| anyhow!("JSON parse failed ({}): {}", e, raw_text))?;

            tracing::info!("✅ DMC analysis succeeded via {model}");
            return Ok(match raw {
                RawScreenshot::Results { boss_name, dmg_pts, boss_pts, has_bonus } =>
                    ScreenshotData::Results { boss_name, dmg_pts, boss_pts, has_bonus },
                RawScreenshot::Leaderboard { boss_name, has_bonus, players } =>
                    ScreenshotData::Leaderboard { boss_name, has_bonus, players },
            });
        }
        // all retries exhausted for this model — try next
    }
    Err(last_err)
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

/// Remove ```json ... ``` or ``` ... ``` wrappers that Gemini sometimes adds.
fn strip_json_fences(s: &str) -> &str {
    let s = s.trim();
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
