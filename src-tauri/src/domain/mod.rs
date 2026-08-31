//! Types that cross the IPC boundary. Every one derives `ts_rs::TS` and exports to
//! `src/ipc/types.gen.ts`; hand-written TS for IPC payloads is banned (docs/02 §6).
//! `cargo test export_bindings` regenerates the file; CI fails on a diff.

use serde::{Deserialize, Serialize};

pub mod ai;
pub mod illustrator;
pub mod export;
pub mod file_modifier;
pub mod job;
pub mod locator;
pub mod mcp;
pub mod project;
pub mod search;
pub mod settings;
pub mod source;
pub mod studio;
pub mod viewer;

pub use job::*;

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub build_date: String,
    pub data_dir: String,
    pub license: String,
}

/// A stable colour slot for project cards (resolved to a token on the frontend).
/// Consumed by the projects service in Phase 1.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, ts_rs::TS, PartialEq, Eq)]
#[ts(export, export_to = "types.gen.ts")]
pub enum ProjectColor {
    #[default]
    #[serde(rename = "accent-1")]
    Accent1,
    #[serde(rename = "accent-2")]
    Accent2,
    #[serde(rename = "accent-3")]
    Accent3,
    #[serde(rename = "accent-4")]
    Accent4,
    #[serde(rename = "accent-5")]
    Accent5,
}
