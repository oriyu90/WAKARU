use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ImagesToPdfInput {
    pub images: Vec<String>,
    /// "a4" | "a3" | "letter" | "fit"
    pub page_size: String,
    /// "auto" | "portrait" | "landscape"
    pub orientation: String,
    /// "none" | "sm" | "md" | "lg"
    pub margin: String,
    /// "contain" | "cover"
    pub fit: String,
    pub dest_path: String,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SaveTextInput {
    pub content: String,
    pub dest_path: String,
    /// "md" | "txt"
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TextToMarkdownInput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct WrittenFile {
    pub path: String,
}
