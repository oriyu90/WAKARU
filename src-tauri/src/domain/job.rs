// Job kinds/statuses are the full P0–P11 surface; not every variant is constructed
// until the phase that produces it.
#![allow(dead_code)]

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Ingest,
    Transcribe,
    Embed,
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub project_id: Option<String>,
    pub source_id: Option<String>,
    pub phase: Option<String>,
    pub done: u32,
    pub total: u32,
    pub message: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
}

/// Payload of the `job://progress` event (docs/02 §5).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub job_id: String,
    pub project_id: Option<String>,
    pub source_id: Option<String>,
    pub kind: JobKind,
    pub phase: String,
    pub done: u32,
    pub total: u32,
    pub message: Option<String>,
}
