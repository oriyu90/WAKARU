use crate::domain::visual::{TextWindow, VisualCreate, VisualPreview};
use crate::error::AppResult;
use crate::services::{projects, viewer, visuals};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn visual_create(state: State<'_, AppState>, input: VisualCreate) -> AppResult<VisualPreview> {
    let project_id = input.project_id.clone();
    visuals::create(&state.projects_dir, &project_id, &input)
}

#[tauri::command]
pub fn visual_get(
    state: State<'_, AppState>,
    project_id: String,
    visual_id: String,
) -> AppResult<VisualPreview> {
    visuals::get(&state.projects_dir, &project_id, &visual_id)
}

#[tauri::command]
pub fn visual_list_for_message(
    state: State<'_, AppState>,
    project_id: String,
    message_id: String,
) -> AppResult<Vec<VisualPreview>> {
    visuals::list_for_message(&state.projects_dir, &project_id, &message_id)
}

#[tauri::command]
pub fn source_read_window(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
    offset: u64,
    limit: u64,
) -> AppResult<TextWindow> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::read_text_window(
        &db,
        &state.projects_dir,
        &project_id,
        &source_id,
        offset,
        limit,
    )
}
