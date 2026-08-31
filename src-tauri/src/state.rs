//! Process-wide state, managed by Tauri and reachable from every command via
//! `tauri::State<AppState>`.

use crate::error::{AppError, AppResult};
use crate::jobs::JobRegistry;
use crate::services::ai::StreamRegistry;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AppState {
    /// `app.db` for foreground command handlers. Background jobs open their own
    /// connection to the same file (WAL allows it) via `app_db_path`.
    pub app_db: Mutex<Connection>,
    pub app_db_path: PathBuf,
    pub jobs: Arc<JobRegistry>,
    pub streams: Arc<StreamRegistry>,
    pub data_dir: PathBuf,
    pub projects_dir: PathBuf,
}

impl AppState {
    pub fn with_db<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self
            .app_db
            .lock()
            .map_err(|_| AppError::internal("app_db mutex poisoned"))?;
        f(&conn)
    }
}
