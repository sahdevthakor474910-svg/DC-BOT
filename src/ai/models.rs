use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GeminiChatRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenConfig>,
}

#[derive(Debug, Serialize)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
pub struct GeminiGenConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct GeminiChatResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
    pub error: Option<GeminiApiError>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiCandidateContent,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidateContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiApiError {
    pub message: String,
    pub code: Option<i64>,
}

/// Server management actions that Gemini can produce for the server owner
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ServerAction {
    CreateChannel {
        name: String,
        #[serde(default)]
        kind: Option<String>, // "text", "voice", "category"
        #[serde(default)]
        topic: Option<String>,
    },
    DeleteChannel {
        name: String,
    },
    RenameChannel {
        old_name: String,
        new_name: String,
    },
    SetSlowmode {
        #[serde(default)]
        channel: Option<String>,
        seconds: u16,
    },
    KickUser {
        user: String,
        #[serde(default)]
        reason: Option<String>,
    },
    BanUser {
        user: String,
        #[serde(default)]
        reason: Option<String>,
    },
}
