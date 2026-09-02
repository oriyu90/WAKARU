use super::source::SourceKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ViewerTab {
    pub id: String,
    pub source_id: String,
    pub kind: SourceKind,
    pub name: String,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
    pub pinned: bool,
    pub ordinal: i32,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct OpenTabInput {
    pub project_id: String,
    pub source_id: String,
    #[serde(default)]
    #[ts(type = "unknown | null")]
    pub locator: Option<serde_json::Value>,
}

/// One page/slide/segment/sheet/section returned by `source_get_document`.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DocumentPayload {
    pub source_id: String,
    pub ordinal: u32,
    pub total: u32,
    pub kind: String,
    pub title: Option<String>,
    pub text: String,
    /// `wakaru-asset://` URL for the page image, when one exists.
    pub image_url: Option<String>,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
}

/// Lightweight source detail for a preview header (name, kind, page count,
/// the reader-view asset URL for web links, etc.).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SourceDetail {
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    pub url: Option<String>,
    pub page_count: Option<u32>,
    pub status: super::source::SourceStatus,
    pub mime: Option<String>,
    pub bytes: u64,
    /// `wakaru-asset://` URL to the primary rendered/original file, if any:
    /// the normalised image, saved web reader markdown, or original imported
    /// file. The URL is project-scoped and never exposes an absolute path.
    pub primary_asset_url: Option<String>,
}
