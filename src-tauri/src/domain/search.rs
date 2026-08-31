use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum SearchScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub scope: SearchScope,
    #[serde(default)]
    pub project_id: Option<String>,
    pub q: String,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// "keyword" | "semantic" | "hybrid" (docs/06 §7). Ignored for global scope.
    #[serde(default)]
    pub mode: Option<String>,
}

fn default_limit() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub project_id: String,
    pub project_name: String,
    pub source_id: String,
    pub source_name: String,
    pub document_id: Option<String>,
    pub ordinal: Option<u32>,
    pub snippet: String,
    #[ts(type = "unknown")]
    pub locator: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    /// true when a semantic component ran (vs. FTS-only fallback).
    pub semantic: bool,
}
