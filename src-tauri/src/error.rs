//! The single error type crossing the IPC boundary (docs/02 §4).
//!
//! `message` is an English technical string for logs; `i18n_key` is what the UI
//! translates. `code` is a stable identifier tests and the frontend switch on.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub i18n_key: String,
    #[ts(type = "unknown | null")]
    pub details: Option<serde_json::Value>,
    pub retriable: bool,
}

impl AppError {
    pub fn new(code: &str, i18n_key: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            i18n_key: i18n_key.to_string(),
            details: None,
            retriable: false,
        }
    }

    #[allow(dead_code)] // used from Phase 3 (AI retry classification)
    pub fn retriable(mut self) -> Self {
        self.retriable = true;
        self
    }

    #[allow(dead_code)] // used from Phase 1 (structured error payloads)
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Generic internal failure. Prefer a specific `code` where the caller can act on it.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL", "error.internal", message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::new("DB", "error.db", e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::new("IO", "error.io", e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::new("SERDE", "error.internal", e.to_string())
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
