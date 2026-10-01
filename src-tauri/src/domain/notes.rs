use serde::{Deserialize, Serialize};

/// A sticky note bound to a source + locator (plan §5). Viewer-private memo;
/// never fed to AI search/prompts automatically.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub source_id: String,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
    pub anchor_kind: String,
    #[ts(type = "unknown")]
    pub anchor_json: serde_json::Value,
    pub body: String,
    pub color: String,
    pub stack_order: i32,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct NoteCreate {
    pub project_id: String,
    pub source_id: String,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub locator: Option<serde_json::Value>,
    #[serde(default = "default_anchor_kind")]
    pub anchor_kind: String,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub anchor_json: Option<serde_json::Value>,
    #[serde(default)]
    pub body: String,
    #[serde(default = "default_note_color")]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct NoteUpdate {
    pub project_id: String,
    pub note_id: String,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub locator: Option<serde_json::Value>,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub anchor_json: Option<serde_json::Value>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub stack_order: Option<i32>,
    /// Last-known `updated_at` for conflict detection; mismatch → NOTE_CONFLICT.
    #[serde(default)]
    pub expected_updated_at: Option<String>,
}

fn default_anchor_kind() -> String {
    "page".to_string()
}

fn default_note_color() -> String {
    "yellow".to_string()
}
