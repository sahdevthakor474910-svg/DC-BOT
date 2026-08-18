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

/// A single boss entry from the boss selection / overview screen.
#[derive(Debug, Deserialize)]
pub struct BossOverviewEntry {
    pub boss_name: String,
    /// The PTS value displayed beneath the boss card.
    pub pts: i64,
    /// Whether this boss card shows a Bonus indicator (e.g. "Bonus +20%").
    pub has_bonus: bool,
}

/// The three kinds of screenshots the bot can receive.
#[derive(Debug)]
pub enum ScreenshotData {
    /// Post-battle results screen (shows DMG PTS / Reward PTS / Boss PTS).
    Results {
        boss_name: String,
        dmg_pts: i64,
        /// Time-bonus Reward PTS read directly from the results screen.
        /// Using this avoids having to compute boss_pts - dmg_pts, which
        /// breaks for Dante / HC / Vergil where both numbers are ~2.89B.
        reward_pts: i64,
        boss_pts: i64,
        has_bonus: bool,
    },
    /// Leaderboard / ranking screen.
    Leaderboard {
        boss_name: String,
        has_bonus: bool,
        players: Vec<LeaderboardPlayer>,
    },
    /// Boss selection / overview screen showing multiple bosses and their PTS.
    BossOverview {
        bosses: Vec<BossOverviewEntry>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw Gemini JSON shapes (intermediate deserialization)
// ─────────────────────────────────────────────────────────────────────────────

/// Gemini can return any of the three screen types; we deserialise via a "type" tag.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawScreenshot {
    Results {
        boss_name: String,
        dmg_pts: i64,
        /// Reward PTS read directly from the results screen (optional for
        /// backwards-compat — falls back to 0 so older prompts still parse).
        #[serde(default)]
        reward_pts: i64,
        boss_pts: i64,
        has_bonus: bool,
    },
    Leaderboard {
        boss_name: String,
        has_bonus: bool,
        players: Vec<LeaderboardPlayer>,
    },
    #[serde(rename = "bossoverview")]
    BossOverview {
        bosses: Vec<BossOverviewEntry>,
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

const ANALYSIS_PROMPT: &str = r#"You are analyzing Devil May Cry: Peak of Combat (DMC:PoC) battle screenshots.

First identify the screenshot type:
- "results"      = personal post-battle screen showing DMG PTS / Reward PTS / Boss PTS
- "leaderboard"  = server ranking screen showing player names and Total PTS
- "bossoverview" = boss selection screen showing multiple boss cards, each with a name and PTS score

═══════════════════════════════════
RESULTS SCREENSHOT — Field Definitions
═══════════════════════════════════
The results screen shows exactly THREE score rows with these labels:
  DMG PTS    — raw damage dealt to the boss HP bar  (large number)
  Reward PTS — time-bonus for finishing quickly      (usually much smaller)
  Boss PTS   — DMG PTS + Reward PTS combined         (largest number)

Extract ALL FOUR of these fields:
1. boss_name  — e.g. "Dante", "Vergil", "Hell Commander", "Calibur"
2. dmg_pts    — the number on the "DMG PTS" row
3. reward_pts — the number on the "Reward PTS" row  ← READ THIS DIRECTLY
4. boss_pts   — the number on the "Boss PTS" row
5. has_bonus  — true if boss is in the BONUS list below, else false

CRITICAL: reward_pts is always MUCH SMALLER than dmg_pts for big bosses like
Dante, Vergil, Hell Commander. Do NOT confuse it with DMG PTS.
If reward_pts appears larger than dmg_pts you have the rows swapped.

Example 1 — Non-bonus boss (Calibur):
{
  "type": "results",
  "boss_name": "Calibur",
  "dmg_pts": 1022497809,
  "reward_pts": 11295415,
  "boss_pts": 1033793224,
  "has_bonus": false
}

Example 2 — Bonus boss (Dante, mid-clear with time left):
{
  "type": "results",
  "boss_name": "Dante",
  "dmg_pts": 2400000000,
  "reward_pts": 8540200,
  "boss_pts": 2408540200,
  "has_bonus": true
}

Example 3 — Bonus boss (Hell Commander, full-clear with time left):
{
  "type": "results",
  "boss_name": "Hell Commander",
  "dmg_pts": 2892440140,
  "reward_pts": 1881540,
  "boss_pts": 2894321680,
  "has_bonus": true
}

═══════════════════════════════════
LEADERBOARD SCREENSHOT
═══════════════════════════════════
Extract:
1. boss_name — the boss tab that is highlighted/selected on the left side
2. has_bonus — true if boss is in the BONUS list below
3. All visible players with rank, name, total_pts

CRITICAL DIGIT COUNT RULE FOR LEADERBOARDS:
- Count ALL digits in player scores extremely carefully! Scores in DMC: PoC leaderboards are 10-DIGIT NUMBERS (e.g. 8691984342, 8690908101, 1033499653).
- Do NOT drop the last digit of any score! Rank 1 scores are often ~8.69 billion (10 digits like 8691984342), NOT 9 digits (like 869198432).
- Verify that a higher-ranked player's total_pts is >= the player below them!

Example:
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
BOSS OVERVIEW SCREENSHOT
═══════════════════════════════════
This screen shows multiple boss cards side-by-side. Each card has:
- A boss name (displayed in stylised text at the bottom of the card)
- A PTS value displayed as "PTS:XXXXXXXXX" beneath the card
- An optional "Bonus" label (e.g. "Bonus ↑20%" or "Bonus +20%") in the top corner

Extract ALL visible boss cards.
For each boss:
1. boss_name — the name on the card (e.g. "Nevan", "Cerberus", "Calibur")
2. pts       — the integer after "PTS:" (no commas)
3. has_bonus — true ONLY if a Bonus indicator is visibly shown on that card

Example (3 bosses visible, one with bonus):
{
  "type": "bossoverview",
  "bosses": [
    {"boss_name": "Nevan",    "pts": 874958218,    "has_bonus": false},
    {"boss_name": "Cerberus", "pts": 1347104169,   "has_bonus": false},
    {"boss_name": "Calibur",  "pts": 1240079805,   "has_bonus": true}
  ]
}

═══════════════════════════════════
RULES FOR ALL SCREEN TYPES
═══════════════════════════════════
- All numbers must be plain integers — no commas, no spaces
- has_bonus true ONLY if the boss is in the BONUS list OR a Bonus label is visible on that card
- Extract ALL visible players / boss cards
- If a value is unreadable, use 0

BONUS BOSSES (X120% multiplier applied by server):
Nevan, Hell Shade, Beowulf, Plutone, Vergil, Dante,
Hell Commander, Hell·Commander, Hell-Commander, Hell-commander

NON-BONUS BOSSES (no multiplier):
Devil Mite, Cerberus, Minotaur, Phantom, Calibur"#;

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
                RawScreenshot::Results { boss_name, dmg_pts, reward_pts, boss_pts, has_bonus } =>
                    ScreenshotData::Results { boss_name, dmg_pts, reward_pts, boss_pts, has_bonus },
                RawScreenshot::Leaderboard { boss_name, has_bonus, players } =>
                    ScreenshotData::Leaderboard { boss_name, has_bonus, players },
                RawScreenshot::BossOverview { bosses } =>
                    ScreenshotData::BossOverview { bosses },
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
