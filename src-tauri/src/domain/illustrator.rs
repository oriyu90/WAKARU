use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    Simple,
    Standard,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Page,
    Source,
    Project,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub source_id: String,
    pub source_name: String,
    pub document_id: Option<String>,
    pub chunk_id: Option<String>,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[ts(type = "Citation[]")]
    pub citations: serde_json::Value,
    pub model: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    pub scope: String,
    pub source_id: Option<String>,
    pub locator_key: Option<String>,
    pub title: String,
    pub messages: Vec<ChatMessage>,
}

/// The saved page explanation (docs/03 §4 `illustrations`).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Illustration {
    pub content: String,
    #[ts(type = "Citation[]")]
    pub citations: serde_json::Value,
    pub level: DetailLevel,
    pub model: String,
    pub created_at: String,
    /// true when this came from the cache (docs/06 §5.2 — "保存された解説").
    pub cached: bool,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct GenerateInput {
    pub project_id: String,
    pub source_id: String,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
    pub level: DetailLevel,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AskInput {
    pub project_id: String,
    pub thread_id: String,
    pub text: String,
    pub scope: Scope,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ImportToStudioInput {
    pub project_id: String,
    pub thread_id: String,
    /// "new_tab" | "append"
    pub mode: String,
    #[serde(default)]
    pub target_tab_id: Option<String>,
}

/// `stream://citations` payload.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StreamCitations {
    pub stream_id: String,
    pub message_id: String,
    #[ts(type = "Citation[]")]
    pub citations: serde_json::Value,
}
