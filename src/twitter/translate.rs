use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

const MYMEMORY_URL: &str = "https://api.mymemory.translated.net/get";

#[derive(Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: ResponseData,
    #[serde(rename = "responseStatus")]
    response_status: u16,
}

#[derive(Deserialize)]
struct ResponseData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

/// Translate text from Japanese to English using the MyMemory free API.
/// Returns the original text unchanged if translation fails.
pub async fn translate_ja_to_en(client: &Client, text: &str) -> String {
    match try_translate(client, text).await {
        Ok(translated) => translated,
        Err(e) => {
            debug!("Translation failed, using original: {}", e);
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

async fn try_translate(client: &Client, text: &str) -> Result<String> {
    // MyMemory has a 500-char limit per request — truncate safely
    let truncated = truncate_to_char_boundary(text, 490);

    let resp = client
        .get(MYMEMORY_URL)
        .query(&[("q", truncated), ("langpair", "ja|en")])
        .send()
        .await?
        .error_for_status()?;

    let body: MyMemoryResponse = resp.json().await?;

    if body.response_status != 200 {
        anyhow::bail!("MyMemory returned status {}", body.response_status);
    }

    Ok(body.response_data.translated_text)
}

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
