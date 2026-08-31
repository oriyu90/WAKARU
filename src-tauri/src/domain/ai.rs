use serde::{Deserialize, Serialize};

/// Wire protocol used by an AI connection. Both variants are implemented
/// directly on reqwest; no vendor SDK is linked into the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum ApiProtocol {
    Openai,
    Anthropic,
}

impl ApiProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

/// A model role (docs/03 §3 `model_roles`). `organizer` falls back to `chat`,
/// `embedding` falls back to local (Phase 3+: to "none / FTS only").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Chat,
    Vision,
    Embedding,
    Organizer,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AiProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub protocol: ApiProtocol,
    /// The API key is never returned — only whether one is stored.
    pub has_key: bool,
    pub default_model: Option<String>,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_embed: bool,
    pub json_schema: bool,
    #[ts(type = "Record<string, string>")]
    pub extra_headers: serde_json::Value,
    pub timeout_ms: u32,
    pub created_at: String,
    pub last_ok_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AiProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_protocol")]
    pub protocol: ApiProtocol,
    /// Present only when the user typed or changed it. `""` clears the key.
    pub api_key: Option<String>,
    pub default_model: Option<String>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | null")]
    pub extra_headers: Option<serde_json::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

fn default_protocol() -> ApiProtocol {
    ApiProtocol::Openai
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub models: Vec<String>,
    pub latency_ms: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_embed: bool,
    pub json_schema: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct RoleBinding {
    pub profile_id: String,
    pub model: String,
    #[ts(type = "Record<string, unknown>")]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct RoleBindings {
    pub chat: Option<RoleBinding>,
    pub vision: Option<RoleBinding>,
    pub embedding: Option<RoleBinding>,
    pub organizer: Option<RoleBinding>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// `stream://delta` payload (docs/02 §5).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StreamDelta {
    pub stream_id: String,
    /// "text" | "reasoning"
    pub kind: String,
    pub text: String,
}

/// `stream://done` payload.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StreamDone {
    pub stream_id: String,
    pub cancelled: bool,
    pub truncated: bool,
    pub usage: Option<TokenUsage>,
}

/// `stream://error` payload.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StreamError {
    pub stream_id: String,
    pub error: crate::error::AppError,
}
