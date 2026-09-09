use crate::domain::source::*;
use crate::error::AppResult;
use crate::services::{projects, sources};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn source_list(state: State<'_, AppState>, project_id: String) -> AppResult<Vec<Source>> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    sources::list(&db)
}

#[tauri::command]
pub fn source_get(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
) -> AppResult<Source> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    sources::get(&db, &source_id)
}

#[tauri::command]
pub fn source_add_files(
    app: AppHandle,
    state: State<'_, AppState>,
    input: AddFilesInput,
) -> AppResult<Vec<Source>> {
    sources::add_files(
        &app,
        &state.app_db_path,
        &state.projects_dir,
        state.jobs.clone(),
        &input.project_id,
        input.paths,
    )
}

#[tauri::command]
pub fn source_add_url(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    url: String,
) -> AppResult<crate::domain::source::Source> {
    sources::add_url(
        &app,
        &state.app_db_path,
        &state.projects_dir,
        state.jobs.clone(),
        &project_id,
        &url,
    )
}

#[tauri::command]
pub fn source_add_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
) -> AppResult<Source> {
    sources::add_folder(
        &app,
        &state.app_db_path,
        &state.projects_dir,
        state.jobs.clone(),
        &project_id,
        &folder,
    )
}

#[tauri::command]
pub fn source_reanalyze(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
) -> AppResult<()> {
    sources::reanalyze(
        &app,
        &state.app_db_path,
        &state.projects_dir,
        state.jobs.clone(),
        &project_id,
        &source_id,
    )
}

#[tauri::command]
pub fn source_delete(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
) -> AppResult<()> {
    state.with_db(|db| sources::delete(db, &state.projects_dir, &project_id, &source_id))
}
