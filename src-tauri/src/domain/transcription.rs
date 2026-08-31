//! Audio / video transcription types (docs/04 §2, docs/05, AC-5-*). Whisper
//! models are downloaded on demand, verified, and selectable; a transcript is a
//! list of time-stamped segments that become `documents` rows.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct WhisperModel {
    /// `"base" | "small" | "medium" | "large-v2"`.
    pub name: String,
    /// Rough download size, for the pre-flight disk check (AC-5-2).
    #[ts(type = "number")]
    pub approx_bytes: u64,
    pub downloaded: bool,
    /// SHA-256 of the downloaded file, recorded after the first successful
    /// download (DECISIONS D-15 — no authoritative upstream manifest to pin to).
    pub sha256: Option<String>,
    /// Absolute path once downloaded.
    pub path: Option<String>,
    /// True for the model currently bound in settings.
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DiskCheck {
    pub ok: bool,
    #[ts(type = "number")]
    pub needed_bytes: u64,
    #[ts(type = "number")]
    pub free_bytes: u64,
}

/// One line of a transcript. `start` / `end` are seconds from the top of the
/// media (AC-5-3, AC-5-8).
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}
