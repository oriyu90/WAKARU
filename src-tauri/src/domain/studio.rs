//! Studio types (docs/06 §6, FR-T1..T7). A Studio tab is one `threads` row
//! (`scope = 'studio'`) plus a `studio_tabs` row for ordering, title and the
//! retrieval scope. Chat turns live in `messages`; files the model writes land
//! in `workspace/` and get an `artifacts` row.

use super::illustrator::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StudioTab {
    pub id: String,
    pub thread_id: String,
    pub title: String,
    pub ordinal: u32,
    /// `"project"` or `"source:<id>"` (docs/06 §6 scope selector).
    pub scope: String,
    /// Total turns in the thread. Always set. `messages` is only filled by
    /// `studio_get_tab` — `studio_list_tabs` returns it empty so the rail does
    /// not pay for every tab's whole history on each poll.
    pub message_count: u32,
    pub messages: Vec<ChatMessage>,
}

/// A file produced in `workspace/` by a Studio tool call (FR-T7). Survives the
/// tab that made it (AC-6-10).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub thread_id: Option<String>,
    pub rel_path: String,
    #[ts(type = "number")]
    pub bytes: u64,
    pub mime: Option<String>,
    pub imported_source_id: Option<String>,
    pub created_at: String,
}

/// Returned by `studio_send` / `studio_resolve_tool`. The frontend refetches
/// tabs + artifacts on success; this is the loop's own summary of what happened.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StudioSendResult {
    /// How many model round-trips the loop ran (0..=10).
    pub iterations: u32,
    /// The loop stopped at the 10-iteration cap with tool calls still pending —
    /// the frontend shows a "続行" button (AC-6-8).
    pub needs_continue: bool,
    /// A `write_file` call is waiting for the reader's approval (AC-6-5 card).
    pub awaiting_approval: bool,
    /// The reader cancelled mid-loop (`studio_cancel`).
    pub cancelled: bool,
    /// Convenience: how many context messages were folded into a summary this
    /// turn (AC-6-9). 0 when nothing was trimmed.
    pub summarised_messages: u32,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StudioSendInput {
    pub project_id: String,
    pub tab_id: String,
    pub text: String,
    /// `"project"` or `"source:<id>"`.
    pub scope: String,
    /// When set, replace this user turn and discard later turns atomically
    /// before generating the new branch.
    #[serde(default)]
    pub replace_from_message_id: Option<String>,
    /// Session-only model override: a profile id whose default model answers
    /// this turn instead of the chat role binding. Never persisted, so
    /// reopening the tab falls back to the configured default.
    #[serde(default)]
    pub model_profile_id: Option<String>,
}
