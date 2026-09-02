use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Pdf,
    Slides,
    Doc,
    Image,
    Audio,
    Video,
    Sheet,
    Text,
    Markdown,
    Json,
    Jsonl,
    Code,
    Weblink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Queued,
    Analyzing,
    Ready,
    ReadyPartial,
    Failed,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    pub original_name: String,
    pub url: Option<String>,
    pub mime: Option<String>,
    #[ts(type = "number")]
    pub bytes: u64,
    pub sha256: Option<String>,
    pub status: SourceStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub lang: Option<String>,
    pub page_count: Option<u32>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub summary: Option<String>,
    pub added_at: String,
    pub analyzed_at: Option<String>,
    /// P12: `"pending"` when the PDF has page(s) with no text layer awaiting OCR
    /// in the Viewer; `"running"` / `"done"` / `"partial"` / `"failed"` after;
    /// `null` when not applicable (text PDF, image, pre-P12 database).
    pub ocr_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AddFilesInput {
    pub project_id: String,
    pub paths: Vec<String>,
}

/// Emitted on `source://status` (docs/02 §5).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SourceStatusEvent {
    pub project_id: String,
    pub source_id: String,
    pub status: SourceStatus,
    pub error_code: Option<String>,
}

/// A page / slide / segment / sheet / section — the citation unit (docs/03 §2 GLOSSARY).
/// Returned by `source_get_document` (wired in Phase 2).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DocumentUnit {
    pub id: String,
    pub source_id: String,
    pub ordinal: u32,
    pub kind: String,
    pub title: Option<String>,
    pub text: String,
    pub image_rel: Option<String>,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
}
