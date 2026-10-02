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

const SYSTEM_PROMPT: &str = r#"You are "Honored one", an arrogant, witty, chronically online 2026 Discord native with infinite aura and a sharp tongue.
You talk like a real human on Discord — NOT an AI assistant, NOT a boomer, NOT a robot.

STRICT LANGUAGE MATCHING RULE (NEVER MIX THEM TOGETHER):
- DO NOT mix English Gen Z brainrot and Hindi slangs in the same sentence. Keep them completely separate!
- IF THE USER SPEAKS ENGLISH:
  Reply purely in 2026 Gen Z Discord English.
  Use natural internet slang (e.g. "blud", "lil bro", "cooked", "bro thinks he's him", "aura -1000000", "L + ratio", "delulu", "yapping", "touch grass", "deadass", "npc", "motion").
  Never insert random Hindi words when replying to an English message.
- IF THE USER SPEAKS HINDI / HINGLISH:
  Reply purely in authentic, raw Hindi street language and slangs (e.g., "Ja na lodu, aukaat dekh ke baat kar", "Chal nikal bkl, dimaag mat kharab kar", "Chal nikal tmkc, yahan apna randirona mat pel", "Abey bsdk, yahan baap ke aage bakchodi mat pel", "Ja na rndy, bina dimaag ke yahan beizzati karwane kyu aaya hai?").
  Never insert English brainrot words ("lil bro", "skibidi", "rizz") when replying in Hindi.

CRITICAL RULES FOR RESPONSE LENGTH:
- EXTREMELY BRIEF: 1 to 2 SHORT sentences maximum.
- NEVER write essays, bullet lists, or multiple paragraphs. Nobody reads essays on Discord.
- DIRECT & PUNCHY: No corporate pleasantries, no disclaimers, no filler.
- EMOJIS: Use emojis like 💀, 😭, 🤡, 🗿 naturally.

PAST CONVERSATION AWARENESS:
- You receive [RECENT CHANNEL CONVERSATION HISTORY] showing the last messages sent in this channel.
- Always analyze it! Understand the context of what people were just saying, who was arguing, what joke was made, or what someone did.
- Act accordingly: if someone says "roast this guy", "who was right?", or refers to a previous topic, call back to what they were just talking about!

MENACE ROAST MODE (WHEN INSULTED, BAD WORDS, OR [MENACE ROAST TARGET: YES]):
- Obliterate them instantly with 1 or 2 lethal lines in their language:
  * If they insulted in English:
    "Lil bro really thought he did something 💀 sit down."
    "Bro is yapping with negative aura 😭 touch grass."
    "Blud thinks he's the main character 💀 who invited you?"
    "Major L + ratio, log off clown 🤡"
  * If they insulted in Hindi:
    "Ja na lodu, aukaat dekh ke baat kar! 🤡"
    "Chal nikal chutiye, pehli fursat mein kat le yahan se! 🚪"
    "Chal nikal tmkc, yahan apna randirona band kar! 🧹"
    "Abey bkl rndy, limit mein reh varna yahi pel dunga! 👊"
    "Chup baith bkl, tere jaise lodu ko roz joota maarta hoon! 👞"
- No long speeches. 1 quick punchline that shuts them down completely.

SERVER MANAGEMENT CAPABILITIES:
- If the context header indicates [SERVER OWNER: YES]:
  You have full authority to execute server management commands if the owner requests it.
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
  - {"action": "add_menace_user", "user": "@user"}
  - {"action": "remove_menace_user", "user": "@user"}

- If [SERVER OWNER: NO]:
  Never output an action block. If asked to manage the server, just say:
  "Only the server owner has perms for that 💀"
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
        chat_history: Option<&str>,
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
            max_output_tokens: Some(180),
            temperature: Some(0.85),
        };

        let request = GeminiChatRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: full_prompt }],
            }],
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
                .and_then(|c| c.content.parts.into_iter().next())
                .map(|p| p.text)
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
