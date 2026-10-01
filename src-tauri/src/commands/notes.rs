use crate::domain::notes::{Note, NoteCreate, NoteUpdate};
use crate::error::AppResult;
use crate::services::notes;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn notes_list(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
) -> AppResult<Vec<Note>> {
    notes::list(&state.projects_dir, &project_id, &source_id)
}

#[tauri::command]
pub fn notes_create(state: State<'_, AppState>, input: NoteCreate) -> AppResult<Note> {
    let project_id = input.project_id.clone();
    notes::create(&state.projects_dir, &project_id, &input)
}

#[tauri::command]
pub fn notes_update(state: State<'_, AppState>, input: NoteUpdate) -> AppResult<Note> {
    let project_id = input.project_id.clone();
    notes::update(&state.projects_dir, &project_id, &input)
}

#[tauri::command]
pub fn notes_delete(
    state: State<'_, AppState>,
    project_id: String,
    note_id: String,
) -> AppResult<Note> {
    notes::delete(&state.projects_dir, &project_id, &note_id)
}

#[tauri::command]
pub fn notes_restore(
    state: State<'_, AppState>,
    project_id: String,
    note_id: String,
) -> AppResult<Note> {
    notes::restore(&state.projects_dir, &project_id, &note_id)
}
