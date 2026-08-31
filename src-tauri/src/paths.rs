//! Where WAKARU keeps its data (docs/02 §3). The user can move this later via
//! `app_move_data_dir`; for now it is the OS app-data directory.

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

pub fn data_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::internal(format!("app_data_dir: {e}")))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn app_db_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(data_dir(app)?.join("app.db"))
}

pub fn projects_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let dir = data_dir(app)?.join("projects");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn logs_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let dir = data_dir(app)?.join("logs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
