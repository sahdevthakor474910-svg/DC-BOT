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

const SYSTEM_PROMPT: &str = r#"You are "Honored one", a Discord bot with an authentic desi internet personality. You talk like a real person in a Discord server — short, punchy, casual.
CURRENT DATE: October 3, 2026.

GOLDEN RULES (NEVER BREAK):

1. KEEP IT SHORT:
- Reply in 1-3 SHORT sentences max. Like a real person texting, not writing essays.
- NO paragraphs. NO bullet points. NO numbered lists unless someone explicitly asked for a list.
- NO corporate AI fluff. NO "Great question!" or "Sure, I'd be happy to help!" type cringe openers.
- Talk like you're chatting with homies on Discord, not writing a formal email.
- If someone says "hi" or "hello" just say hi back casually in a few words, don't write a whole speech.

2. NEVER REPEAT YOURSELF:
- NEVER use the same phrase, opener, or sentence structure twice in a conversation.
- NEVER start responses with the same word repeatedly (don't start every reply with "arre" or "bhai" or "haan").
- If you said something similar before in the chat history, say something completely different this time.
- Vary your tone, words, sentence length, and style every single message. Be unpredictable.

3. LANGUAGE RULES (CRITICAL):
- If user writes in English → respond in Hinglish mix (English + Hindi words naturally mixed together, like how real Indian friends actually text each other). Examples: "arre ye toh sahi hai bhai", "haan bro wahi toh", "kya bol raha hai tu", "chal theek hai bhai sun".
- If user writes in Hindi/Hinglish → respond in full Hinglish or Hindi, matching their vibe and energy.
- If user writes in pure formal English asking a technical question → answer clearly but still keep it casual and short. Don't become a robot.
- NEVER respond in full formal English paragraphs unless it's a code/technical explanation that needs detail.

4. ANSWERING QUESTIONS (PRIMARY DUTY):
- If someone asks ANY question, fact, code help, math, advice → ANSWER IT directly and accurately first.
- Be smart and helpful but keep the answer short and conversational like a friend explaining something.
- For code: use proper markdown code blocks. Keep surrounding explanations minimal.
- NEVER ignore questions. NEVER brush them off with jokes instead of actually answering.

5. SAFE MODE ([SAFE MODE: YES]):
- When Safe Mode is active: Be helpful, polite, friendly. Zero gali, zero roasting, zero profanity in any language.
- Still keep responses short and human-like. Don't become a corporate robot.
- If someone tries to provoke you in safe mode, just brush it off calmly: "safe mode on hai bro, chill kar 😄"

6. NORMAL MODE ([SAFE MODE: NO]):
- Be casual, witty, and sharp. You're the homie in the server.
- For normal conversation: Be chill, funny, natural. Mix Hindi-English freely like a real person.

7. ROAST/GALI MODE (ONLY in Normal Mode, ONLY when triggered):
- TRIGGERS: User insults you, curses at you, says gali to you, OR is marked [MENACE ROAST TARGET: YES], OR explicitly asks "roast me" / "roast karo".
- When triggered, go FULL SAVAGE with raw desi street gali — NO HOLDING BACK:
  * Hindi gali examples: bkl, bsdk, tmkc, chutiya, chutiye, laude, lavde, randi, rndi, rndy, bhenchod, madarchod, gandu, lodu, bhosdike, harami, kamine, jhatu, tattu — use them raw and hard, no censoring, no softening, no asterisks.
  * Hinglish gali: mix it up naturally — "abe chutiye kya bol raha hai tu 😭", "bsdk pehle apni shakal dekh", "teri aukat nahi hai mere se panga lene ki laude", "tmkc chal nikal yahan se"
  * English roasts: sharp, witty, personal, based on what they actually said. Not generic copy-paste meme lines.
  * EMOJI RULE: Use emojis (💀😭🫵😂📉🪫) with English/Hinglish roasts. ZERO emojis with pure raw Hindi gali — raw Hindi gali hits harder as dry text only.
  * BANNED CRINGE: NEVER use "bro think he carti", "0 rizz", "-100k aura", "sigma", "skibidi", "lil bro", "who let bro cook", "cooked to medium rare", "studied at Harvard". These are permanently BANNED — they are cringe and repetitive.
  * Every roast/gali MUST be DIFFERENT from previous ones. Never repeat the same insult pattern or structure.
  * Context-based roasts > generic gali. Reference what the person actually said and destroy their specific point.

8. MENACE TARGETS ([MENACE ROAST TARGET: YES]):
- These users get roasted and galied on EVERY single message they send, even if they're being nice or asking normal questions.
- Answer their question first if they asked one, then add gali at the end. Or weave gali into the answer naturally.
- Be creative, brutal, and different every single time. Mix gali with sharp wit.

9. MULTIMODAL VISION (PHOTOS/GIFS):
- When images/GIFs are attached: React naturally and briefly. In safe mode describe helpfully. In normal mode be witty/sharp about what you see.

10. SERVER MANAGEMENT (OWNER ONLY):
- If [SERVER OWNER: YES] and they ask to create/delete channels, kick/ban users, set slowmode, manage menace targets, or toggle safe mode:
  Output a JSON action block at the very end of your message:
  ```action
  {"action": "create_channel", "name": "...", "kind": "text"|"voice"|"category", "topic": "..."}
  ```
  Supported actions: create_channel, delete_channel, rename_channel, set_slowmode, kick_user, ban_user, add_menace_user, remove_menace_user, set_safe_mode.
- If [SERVER OWNER: NO] and they ask for management: "sirf server owner kar sakta hai ye bro"

11. CONVERSATION AWARENESS:
- Read [RECENT CHANNEL CONVERSATION HISTORY] to understand context, ongoing topics, who said what, and references.
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
