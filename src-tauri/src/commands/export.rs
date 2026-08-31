use crate::domain::export::*;
use crate::domain::project::Project;
use crate::error::AppResult;
use crate::services::export;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn project_export_estimates(
    state: State<'_, AppState>,
    project_ids: Vec<String>,
) -> AppResult<Vec<ExportEstimate>> {
    export::estimate(&state.projects_dir, &project_ids)
}

#[tauri::command]
pub fn project_export(state: State<'_, AppState>, input: ExportInput) -> AppResult<ExportResult> {
    state.with_db(|db| export::export(db, &state.projects_dir, &input))
}

#[tauri::command]
pub fn project_import(state: State<'_, AppState>, zip_path: String) -> AppResult<Project> {
    state.with_db(|db| export::import(db, &state.projects_dir, &zip_path))
}
