//! Process-wide state, managed by Tauri and reachable from every command via
//! `tauri::State<AppState>`.

use crate::error::{AppError, AppResult};
use crate::jobs::JobRegistry;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    /// `app.db`. Project databases are opened per-call by the projects service.
    pub app_db: Mutex<Connection>,
    pub jobs: JobRegistry,
    pub data_dir: PathBuf,
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
