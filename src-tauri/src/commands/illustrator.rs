use crate::domain::illustrator::*;
use crate::domain::locator::Locator;
use crate::error::AppResult;
use crate::services::{illustrator, projects};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn illustrator_get_or_create_thread(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
    locator: serde_json::Value,
) -> AppResult<Thread> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    illustrator::get_or_create_thread(&db, &source_id, &Locator::key_from_value(&locator))
}

#[tauri::command]
pub async fn illustrator_generate(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: GenerateInput,
    ui_lang: String,
) -> AppResult<illustrator::GenerateStarted> {
    let reg = state.streams.clone();
    let app_db_path = state.app_db_path.clone();
    let projects_dir = state.projects_dir.clone();
    illustrator::generate(&app, reg, &app_db_path, &projects_dir, input, ui_lang).await
}

#[tauri::command]
pub async fn illustrator_ask(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: AskInput,
    ui_lang: String,
) -> AppResult<String> {
    let reg = state.streams.clone();
    let app_db_path = state.app_db_path.clone();
    let projects_dir = state.projects_dir.clone();
    illustrator::ask(&app, reg, &app_db_path, &projects_dir, input, ui_lang).await
}

#[tauri::command]
pub fn illustrator_cancel(state: State<'_, AppState>, stream_id: String) -> AppResult<()> {
    state.streams.cancel(&stream_id);
    Ok(())
}

#[tauri::command]
pub fn illustrator_import_to_studio(
    state: State<'_, AppState>,
    input: ImportToStudioInput,
) -> AppResult<String> {
    let db = projects::open_db(&state.projects_dir, &input.project_id)?;
    illustrator::import_to_studio(&db, &input)
}
