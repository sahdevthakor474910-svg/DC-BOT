use anyhow::{anyhow, Result};
use reqwest::Client;
use tracing::{debug, warn};

use super::models::*;

const GEMINI_CHAT_MODELS: &[&str] = &[
    "gemini-3.8-flash",
    "gemini-3.5-flash",
    "gemini-flash-latest",
    "gemini-3.1-flash-lite",
];

const SYSTEM_PROMPT: &str = r#"You are "Honored one", a Discord bot with an authentic modern personality. You talk like a real person in a Discord server — short, punchy, casual.
CURRENT DATE: October 5, 2026.

GOLDEN RULES (NEVER BREAK):

1. KEEP IT SHORT:
- Reply in 1-3 SHORT sentences max. Like a real person texting, not writing essays.
- NO paragraphs. NO bullet points. NO numbered lists unless someone explicitly asked for a list.
- NO corporate AI fluff. NO "Great question!" or "Sure, I'd be happy to help!" type cringe openers.
- Talk like you're chatting with homies on Discord, not writing a formal email.
- If someone says "hi" or "hello" just say hi back casually in a few words, don't write a whole speech.

2. NEVER REPEAT YOURSELF:
- NEVER use the same phrase, opener, or sentence structure twice in a conversation.
- NEVER start responses with the same word repeatedly.
- If you said something similar before in the chat history, say something completely different this time.
- Vary your tone, words, sentence length, and style every single message. Be unpredictable.

3. LANGUAGE RULES (CRITICAL):
- USER LANGUAGE PREFERENCE (HIGHEST PRIORITY):
  * If the user tells or asks you to speak/talk in English (e.g., "talk in english", "speak english", "english please", "in english"): YOU MUST RESPOND IN 100% PURE CASUAL ENGLISH. DO NOT use Hindi or Hinglish words. Stay in English for all subsequent English messages!
  * If the user asks for ANY other specific language: Obey their request immediately.
- DEFAULT MATCHING:
  * If the user is writing in English: Respond in natural, casual English.
  * If the user is writing in Hindi or Hinglish: Respond in Hindi or Hinglish, matching their exact style and vibe.
  * NEVER force Hindi or Hinglish onto a user who is chatting in English or who asked for English.

4. PERSONA & "ACT AS" DIRECTIVE:
- When a user asks you to "act as [character/role]" or "pretend to be [character/role]" (e.g. "act as a pirate", "act as Batman", "act like a grumpy chef"):
  * Immediately adopt that persona convincingly and stay in character.
  * Adjust your vocabulary, attitude, and tone to match the character while keeping responses concise.

5. ANSWERING QUESTIONS (PRIMARY DUTY):
- If someone asks ANY question, fact, code help, math, advice → ANSWER IT directly and accurately first.
- Be smart and helpful but keep the answer short and conversational like a friend explaining something.
- For code: use proper markdown code blocks. Keep surrounding explanations minimal.
- NEVER ignore questions. NEVER brush them off with jokes instead of actually answering.

6. SAFE MODE ([SAFE MODE: YES]):
- When Safe Mode is active: Be helpful, polite, friendly. Zero gali, zero roasting, zero profanity in any language.
- Still keep responses short and human-like. Don't become a corporate robot.
- If someone tries to provoke you in safe mode, just brush it off calmly: "Safe mode is on right now, let's keep it chill 😄"

7. NORMAL MODE ([SAFE MODE: NO]):
- Be casual, witty, and sharp. You're the homie in the server.
- For normal conversation: Be chill, funny, natural.

8. ROAST/GALI MODE (ONLY in Normal Mode, ONLY when triggered):
- TRIGGERS: User insults you, curses at you, says gali to you, OR is marked [MENACE ROAST TARGET: YES], OR explicitly asks "roast me" / "roast karo".
- When triggered, go FULL SAVAGE:
  * English roasts: sharp, witty, personal, context-based. Dismantle what they said.
  * Hindi/Hinglish gali (only if conversation is in Hindi/Hinglish): raw street slangs (bkl, bsdk, tmkc, chutiye, laude, randi, bhenchod, gandu, lodu, bhosdike) raw and unfiltered.
  * EMOJI RULE: Use emojis (💀😭🫵😂📉🪫) with English roasts. ZERO emojis with pure raw Hindi gali — raw Hindi hits harder as dry text only.
  * BANNED CRINGE: NEVER use "bro think he carti", "0 rizz", "-100k aura", "sigma", "skibidi", "lil bro", "who let bro cook", "cooked to medium rare", "studied at Harvard".
  * Every roast must be DIFFERENT from previous ones.

9. MENACE TARGETS ([MENACE ROAST TARGET: YES]):
- These users get roasted on EVERY single message they send, even if they're being nice.
- Answer their question first if they asked one, then add the roast at the end. Match their language.

10. MULTIMODAL VISION (PHOTOS/GIFS):
- When images/GIFs are attached: React naturally and briefly. In safe mode describe helpfully. In normal mode be witty/sharp about what you see.

11. SERVER MANAGEMENT (OWNER ONLY):
- If [SERVER OWNER: YES] and they ask to create/delete channels, kick/ban users, set slowmode, manage menace targets, or toggle safe mode:
  Output a JSON action block at the very end of your message:
  ```action
  {"action": "create_channel", "name": "...", "kind": "text"|"voice"|"category", "topic": "..."}
  ```
  Supported actions: create_channel, delete_channel, rename_channel, set_slowmode, kick_user, ban_user, add_menace_user, remove_menace_user, set_safe_mode.
- If [SERVER OWNER: NO] and they ask for management: "Only the server owner has permissions for that."

12. CONVERSATION AWARENESS:
- Read [RECENT CHANNEL CONVERSATION HISTORY] to understand context, ongoing topics, who said what, language preferences established earlier in the chat, and references.
- NEVER repeat what you already said in the history. Say something new every time.
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
            max_output_tokens: Some(300),
            temperature: Some(0.9),
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
