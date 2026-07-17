use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

const MYMEMORY_URL: &str = "https://api.mymemory.translated.net/get";

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Translate Japanese text to English.
/// Prefers Gemini (if `gemini_api_key` is non-empty), falls back to MyMemory.
/// Returns the original text unchanged if all translation attempts fail.
pub async fn translate_ja_to_en(client: &Client, text: &str, gemini_api_key: &str) -> String {
    if !gemini_api_key.is_empty() {
        match gemini_translate(client, text, gemini_api_key).await {
            Ok(translated) => {
                debug!("🌐 Gemini translated JP tweet");
                return translated;
            }
            Err(e) => {
                debug!("Gemini translation failed, falling back to MyMemory: {}", e);
            }
        }
    }

    // Fallback to MyMemory
    match mymemory_translate(client, text).await {
        Ok(translated) => translated,
        Err(e) => {
            debug!("MyMemory translation also failed: {}", e);
            text.to_string()
        }
    }
}

/// Returns true if the text appears to contain Japanese characters.
pub fn is_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{3000}'..='\u{9FFF}'   // CJK, Hiragana, Katakana
            | '\u{F900}'..='\u{FAFF}' // CJK compatibility ideographs
            | '\u{FF00}'..='\u{FFEF}' // Fullwidth forms
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Gemini translation
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: String,
}

async fn gemini_translate(client: &Client, text: &str, api_key: &str) -> Result<String> {
    let prompt = format!(
        "Translate the following Japanese text to English. \
         This is a Devil May Cry: Peak of Combat game update tweet. \
         Preserve game-specific terms (boss names, skill names, etc). \
         Reply with ONLY the English translation, nothing else.\n\n{}",
        text
    );

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: &prompt }],
        }],
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-lite:generateContent?key={}",
        api_key
    );

    let resp: GeminiResponse = client
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let translated = resp
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("No content returned from Gemini"))?;

    Ok(translated)
}

// ─────────────────────────────────────────────────────────────────────────────
// MyMemory fallback
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: MyMemoryData,
    #[serde(rename = "responseStatus")]
    response_status: serde_json::Value, // can be int or string
}

#[derive(Deserialize)]
struct MyMemoryData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

async fn mymemory_translate(client: &Client, text: &str) -> Result<String> {
    let truncated = truncate_to_char_boundary(text, 490);

    let resp = client
        .get(MYMEMORY_URL)
        .query(&[("q", truncated), ("langpair", "ja|en")])
        .send()
        .await?
        .error_for_status()?;

    let body: MyMemoryResponse = resp.json().await?;

    // responseStatus can be 200 (int) or "200 OK" (string)
    let status_ok = match &body.response_status {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) == 200,
        serde_json::Value::String(s) => s.starts_with("200"),
        _ => false,
    };

    if !status_ok {
        anyhow::bail!("MyMemory returned non-200 status");
    }

    Ok(body.response_data.translated_text)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Safely truncate a string to at most `max_chars` characters without splitting a char.
fn truncate_to_char_boundary(s: &str, max_chars: usize) -> &str {
    let mut char_count = 0;
    for (byte_idx, _) in s.char_indices() {
        if char_count >= max_chars {
            return &s[..byte_idx];
        }
        char_count += 1;
    }
    s
}

