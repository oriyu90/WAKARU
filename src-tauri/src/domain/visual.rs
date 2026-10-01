use serde::{Deserialize, Serialize};

/// Typed interactive-visual artifact shared by Studio and Live (plan §4.1).
/// Small self-contained HTML/CSS/JS; no remote URLs, CDN, fonts, import(),
/// network fetch or form submission in v1.6.0.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct VisualPreview {
    pub id: String,
    pub schema_version: i32,
    pub title: String,
    pub html: String,
    pub css: String,
    pub js: String,
    #[ts(type = "unknown")]
    pub data_json: serde_json::Value,
    pub aspect_ratio: String,
    #[ts(type = "unknown")]
    pub source_refs: serde_json::Value,
    #[ts(type = "unknown")]
    pub initial_state: serde_json::Value,
    pub model: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct VisualCreate {
    pub project_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub css: String,
    #[serde(default)]
    pub js: String,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub data_json: Option<serde_json::Value>,
    #[serde(default = "default_aspect")]
    pub aspect_ratio: String,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub source_refs: Option<serde_json::Value>,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub initial_state: Option<serde_json::Value>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub message_id: Option<String>,
}

fn default_aspect() -> String {
    "16:9".to_string()
}

/// Windowed text read for huge sources (plan §3.2). UTF-8 safe; never returns
/// more than the requested limit.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TextWindow {
    pub source_id: String,
    pub offset: u64,
    pub total_bytes: u64,
    pub text: String,
    pub next_offset: Option<u64>,
    pub is_truncated: bool,
    /// 1-based line number of the window's first byte (CSV keeps absolute
    /// row numbers; counted by a bounded streaming scan, never loaded).
    pub start_line: u64,
}
