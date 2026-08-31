use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ExportInput {
    pub ids: Vec<String>,
    pub dest_dir: String,
    #[serde(default)]
    pub include_embeddings: bool,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ExportEstimate {
    pub project_id: String,
    pub bytes_without_embeddings: u64,
    pub bytes_with_embeddings: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum VersionVerdict {
    /// same version — open directly
    Accept,
    /// older minor/major — run migrations
    Migrate,
    /// newer minor — open with a warning, keep unknown columns
    WarnOpen,
    /// newer major — refuse
    Reject,
}
