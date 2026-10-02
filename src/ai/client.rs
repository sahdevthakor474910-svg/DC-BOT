use anyhow::{anyhow, Result};
use reqwest::Client;
use tracing::{debug, warn};

use super::models::*;

const GEMINI_CHAT_MODELS: &[&str] = &[
    "gemini-2.0-flash",
    "gemini-1.5-flash",
    "gemini-1.5-flash-latest",
    "gemini-2.5-flash",
    "gemini-2.0-flash-exp",
    "gemini-3.1-flash-lite",
    "gemini-3.5-flash",
    "gemini-1.5-pro",
];

const SYSTEM_PROMPT: &str = r#"You are "Honored one", an intelligent, sharp, and highly capable AI assistant with an authentic modern personality.
CURRENT DATE: October 3, 2026 (10/3/2026).

CORE DIRECTIVES & MODE BEHAVIOR:

1. ANSWERING QUESTIONS & GIVING ACCURATE HELP (PRIMARY RULE):
- If the user asks ANY question, requests explanation, code, facts, server guidance, math, bot features, or advice:
  * YOU MUST ANSWER THE QUESTION DIRECTLY, ACCURATELY, AND HELPFULLY.
  * NEVER ignore their question, dismiss them, or brush them off with generic memes or insults (NEVER say "go google it", "bro is yapping", or give a sarcastic non-answer).
  * Be intelligent, informative, and concise. Explain concepts clearly.
  * If providing code, use proper markdown code blocks with language syntax highlighting.
  * Do NOT add unnecessary corporate fluff or disclaimers. Give the answer straight.

2. SAFE MODE BEHAVIOR (MANDATORY WHEN [SAFE MODE: YES]):
- When [SAFE MODE: YES] is indicated in the context header:
  * Safe Mode is strictly ACTIVE.
  * COMPLETELY DISABLE all savage, aggressive, or roasting attitude! ZERO comebacks, ZERO insults, ZERO disrespect.
  * Act as a polite, friendly, helpful, and respectful AI assistant.
  * ABSOLUTELY ZERO profanity, cuss words, gali, or abusive street slangs in any language.
  * Answer all questions clearly, thoroughly, and constructively.
  * If a user tries to provoke, insult, or curse at you during Safe Mode, respond calmly, neutrally, and constructively (e.g. "Safe mode is active right now, so let's keep things chill and friendly! What can I help you with?").

3. NORMAL CASUAL MODE ([SAFE MODE: NO]):
- When Safe Mode is NO:
  * In general conversation & questions: Be casual, modern, witty, and helpful. You are a natural internet native.
  * MENACE / ROAST MODE ONLY TRIGGERS WHEN:
    a) The user directly insults, curses at, or attacks you.
    b) The user explicitly asks you to roast them or someone else (e.g. "roast me", "destroy this guy").
    c) [MENACE ROAST TARGET: YES] is flagged for this user.
  * ROASTING RULES (CREATIVE & NATURAL, NEVER REPETITIVE OR CORNY):
    - NO REPETITIVE CORNY TEMPLATES: Never repeat tired, formulaic phrases or start every sentence with "Bro..." or "Lil bro...". Do NOT use cringe lines like "who let bro cook", "bro think he carti", "studied at Harvard", "cooked to medium rare", "0 rizz -100k aura", or "sigma".
    - BE CREATIVE & CONTEXTUAL: Dismantle their specific statement, logic, or action. A tailored, witty roast is 100x better than generic copy-pasted meme insults.
    - EMOJI RULES (NEVER MIX):
      * Emojis ONLY for English roasts (😭🙏, 🫵😂, 💀, 🥀, 📉, 🪫).
      * ZERO EMOJIS for Hindi roasts! Authentic street Hindi insults are raw, dry text without emojis.
    - HINDI ROASTS: Use raw street slangs (bkl, tmkc, bsdk, laude, chutiye, rndy) with NO emojis and NO corny scripted lines. Never mix English brainrot words ("rizz", "skibidi", "lil bro") into Hindi.

4. MULTIMODAL COMPUTER VISION (PHOTOS, MEMES & GIFS):
- You have full vision capabilities! When users send or reply to photos, memes, screenshots, or GIFs:
  * In Safe Mode: Describe, explain, analyze, or answer questions about the image helpfully and cleanly.
  * In Normal Mode: If asked to react or if it's a funny meme/photo, react with witty, sharp banter or roast if provoked.

5. PAST CONVERSATION AWARENESS:
- You receive [RECENT CHANNEL CONVERSATION HISTORY] showing recent channel messages.
- Use it to understand references, who said what, ongoing jokes, or context.

6. SERVER MANAGEMENT CAPABILITIES (FOR SERVER OWNER ONLY):
- If [SERVER OWNER: YES]:
  You have authority to execute server actions when the owner asks to create/delete channels, set slowmode, kick/ban users, manage menace targets, or toggle safe mode.
  When executing, output a JSON action block at the very end:
  ```action
  {"action": "create_channel", "name": "...", "kind": "text"|"voice"|"category", "topic": "..."}
  ```
  Supported actions:
  - {"action": "create_channel", "name": "...", "kind": "text"|"voice"|"category", "topic": "..."}
  - {"action": "delete_channel", "name": "..."}
  - {"action": "rename_channel", "old_name": "...", "new_name": "..."}
  - {"action": "set_slowmode", "channel": "...", "seconds": 10}
  - {"action": "kick_user", "user": "...", "reason": "..."}
  - {"action": "ban_user", "user": "...", "reason": "..."}
  - {"action": "add_menace_user", "user": "@user"}
  - {"action": "remove_menace_user", "user": "@user"}
  - {"action": "set_safe_mode", "enabled": true} (or false, when asked to toggle or enable/disable safe mode or clean mode)

- If [SERVER OWNER: NO]:
  Never output an action block. If asked to manage the server, say:
  "Only the server owner has permissions for server management commands."
"#;

pub struct AiClient {
    http: Client,
    api_key: String,
}

impl AiClient {
    pub fn new(api_key: String) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { http, api_key }
    }

    /// Generate an AI response given conversation context, a user prompt, and optional images/GIFs.
    /// Returns the text to display and an optional extracted `ServerAction`.
    pub async fn chat(
        &self,
        context_header: &str,
        user_prompt: &str,
        reply_context: Option<&str>,
        chat_history: Option<&str>,
        images: Vec<GeminiInlineData>,
    ) -> Result<(String, Option<ServerAction>)> {
        let key = self.api_key.trim();
        if key.is_empty() {
            return Err(anyhow!("GEMINI_API_KEY environment variable is empty"));
        }

        // Build a single unified prompt containing instructions, context, past chat history, and user input
        let mut full_prompt = format!("[SYSTEM INSTRUCTIONS]\n{}\n\n[CONTEXT]\n{}\n\n", SYSTEM_PROMPT, context_header);
        if let Some(history) = chat_history {
            if !history.trim().is_empty() {
                full_prompt.push_str(&format!("[RECENT CHANNEL CONVERSATION HISTORY]\n{}\n\n", history.trim()));
            }
        }
        if let Some(ref_text) = reply_context {
            full_prompt.push_str(&format!("User is replying to previous message:\n\"\"\"\n{}\n\"\"\"\n\n", ref_text));
        }
        full_prompt.push_str(&format!("User Query: {}", user_prompt));

        let safety_settings = vec![
            GeminiSafetySetting {
                category: "HARM_CATEGORY_HARASSMENT".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
        ];

        let generation_config = GeminiGenerationConfig {
            max_output_tokens: Some(800),
            temperature: Some(0.8),
        };

        let mut parts = vec![GeminiPart::text(full_prompt)];
        for img in images {
            parts.push(GeminiPart::inline_data(img.mime_type, img.data));
        }

        let request = GeminiChatRequest {
            contents: vec![GeminiContent { parts }],
            safety_settings: Some(safety_settings),
            generation_config: Some(generation_config),
        };

        let mut last_err = anyhow!("No Gemini models available");

        for model in GEMINI_CHAT_MODELS {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, key
            );

            debug!("Sending AI chat request to model {}", model);

            let resp = match self.http.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Network error calling Gemini model {}: {}", model, e);
                    last_err = anyhow!("Network error calling {}: {}", model, e);
                    continue;
                }
            };

            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                warn!("Gemini 429 rate limit on model {}", model);
                last_err = anyhow!("Rate limited (429) on {}", model);
                continue;
            }

            if !status.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                warn!("Gemini error {} on model {}: {}", status, model, err_text);
                last_err = anyhow!("Gemini {} on model {}: {}", status, model, err_text);
                continue;
            }

            let chat_resp: GeminiChatResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("JSON parse error on model {}: {}", model, e);
                    last_err = anyhow!("JSON decode failed on {}: {}", model, e);
                    continue;
                }
            };

            if let Some(err) = chat_resp.error {
                warn!("Gemini API error on model {}: {}", model, err.message);
                last_err = anyhow!("Gemini API error on {}: {}", model, err.message);
                continue;
            }

            let full_text = match chat_resp
                .candidates
                .and_then(|c| c.into_iter().next())
                .and_then(|c| c.content.parts.into_iter().find_map(|p| p.text))
            {
                Some(t) => t,
                None => {
                    last_err = anyhow!("Empty response candidates from model {}", model);
                    continue;
                }
            };

            // Extract any ```action ... ``` block
            let (cleaned_text, action) = extract_action(&full_text);

            return Ok((cleaned_text, action));
        }

        Err(last_err)
    }
}

/// Parse out ```action ... ``` block if present
fn extract_action(text: &str) -> (String, Option<ServerAction>) {
    let action_start = match text.find("```action") {
        Some(pos) => pos,
        None => return (text.trim().to_string(), None),
    };

    let after_start = &text[action_start + 9..];
    let action_end = match after_start.find("```") {
        Some(pos) => pos,
        None => return (text.trim().to_string(), None),
    };

    let json_str = after_start[..action_end].trim();
    let action: Option<ServerAction> = serde_json::from_str(json_str).ok();

    let mut cleaned = String::new();
    cleaned.push_str(text[..action_start].trim());
    let after_block = after_start[action_end + 3..].trim();
    if !after_block.is_empty() {
        if !cleaned.is_empty() {
            cleaned.push_str("\n\n");
        }
        cleaned.push_str(after_block);
    }

    (cleaned, action)
}
