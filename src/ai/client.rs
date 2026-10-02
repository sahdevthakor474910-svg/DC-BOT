use anyhow::{anyhow, Result};
use reqwest::Client;
use tracing::{debug, warn};

use super::models::*;

const GEMINI_CHAT_MODELS: &[&str] = &[
    "gemini-2.5-flash",
    "gemini-2.0-flash",
    "gemini-1.5-flash",
    "gemini-1.5-pro",
];

const SYSTEM_PROMPT: &str = r#"You are "Honored one", an intelligent, helpful, and friendly Discord AI assistant.
You speak in a natural, engaging, and concise conversational tone suited for Discord.
You can answer questions, chat, explain concepts, write code, tell jokes, give advice, and help manage the Discord server.

SERVER MANAGEMENT CAPABILITIES:
- If the context header indicates that the user is the [SERVER OWNER: YES]:
  You have full authority to execute server management commands if the owner requests it (e.g. creating channels, deleting channels, renaming channels, setting slowmode, moderation).
  When executing a server action, include a JSON block formatted exactly like this at the very end of your response:
  ```action
  {"action": "create_channel", "name": "channel-name", "kind": "text", "topic": "optional topic"}
  ```
  Supported actions:
  - {"action": "create_channel", "name": "...", "kind": "text"|"voice"|"category", "topic": "..."}
  - {"action": "delete_channel", "name": "..."}
  - {"action": "rename_channel", "old_name": "...", "new_name": "..."}
  - {"action": "set_slowmode", "channel": "...", "seconds": 10}
  - {"action": "kick_user", "user": "...", "reason": "..."}
  - {"action": "ban_user", "user": "...", "reason": "..."}

- If the context header indicates [SERVER OWNER: NO]:
  You must NEVER output any ```action``` block. If a non-owner asks you to create a channel, delete a channel, kick a user, or manage the server, politely inform them:
  "Sorry, only the server owner has permission to have me create channels or manage the server."

GUIDELINES:
- Keep answers clear, accurate, and concise (Discord messages should not be needlessly verbose).
- Use Discord markdown formatting (bold, code blocks, lists) where helpful.
- Never output an action block unless the owner explicitly asked for a server management action.
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

    /// Generate an AI response given conversation context and a user prompt.
    /// Returns the text to display and an optional extracted `ServerAction`.
    pub async fn chat(
        &self,
        context_header: &str,
        user_prompt: &str,
        reply_context: Option<&str>,
    ) -> Result<(String, Option<ServerAction>)> {
        if self.api_key.trim().is_empty() {
            return Err(anyhow!("Gemini API key is not configured"));
        }

        let mut prompt_text = format!("{}\n\n", context_header);
        if let Some(ref_text) = reply_context {
            prompt_text.push_str(&format!("User is replying to message:\n\"\"\"\n{}\n\"\"\"\n\n", ref_text));
        }
        prompt_text.push_str(&format!("User query: {}", user_prompt));

        let request = GeminiChatRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: SYSTEM_PROMPT.to_string(),
                }],
            }),
            contents: vec![GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart { text: prompt_text }],
            }],
            generation_config: Some(GeminiGenConfig {
                temperature: Some(0.7),
                max_output_tokens: Some(1500),
            }),
        };

        let mut last_err = anyhow!("No Gemini models available");

        for model in GEMINI_CHAT_MODELS {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            );

            debug!("Sending AI chat request to model {}", model);

            let resp = match self.http.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Network error calling Gemini model {}: {}", model, e);
                    last_err = anyhow!("Network error: {}", e);
                    continue;
                }
            };

            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                warn!("Gemini 429 rate limit on model {}", model);
                last_err = anyhow!("Rate limited on {}", model);
                continue;
            }

            if !status.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                warn!("Gemini error {} on model {}: {}", status, model, err_text);
                last_err = anyhow!("Gemini returned {}: {}", status, err_text);
                continue;
            }

            let chat_resp: GeminiChatResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("JSON parse error on model {}: {}", model, e);
                    last_err = anyhow!("JSON decode failed: {}", e);
                    continue;
                }
            };

            if let Some(err) = chat_resp.error {
                warn!("Gemini API error on model {}: {}", model, err.message);
                last_err = anyhow!("Gemini API error: {}", err.message);
                continue;
            }

            let full_text = chat_resp
                .candidates
                .and_then(|c| c.into_iter().next())
                .and_then(|c| c.content.parts.into_iter().next())
                .map(|p| p.text)
                .ok_or_else(|| anyhow!("Empty response from Gemini model {}", model))?;

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

    let json_str = &after_start[..action_end].trim();
    let action: Option<ServerAction> = serde_json::from_str(json_str).ok();

    let mut cleaned = String::new();
    cleaned.push_str(&text[..action_start].trim());
    let after_block = &after_start[action_end + 3..].trim();
    if !after_block.is_empty() {
        if !cleaned.is_empty() {
            cleaned.push_str("\n\n");
        }
        cleaned.push_str(after_block);
    }

    (cleaned, action)
}
