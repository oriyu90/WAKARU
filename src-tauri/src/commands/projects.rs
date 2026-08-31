use crate::domain::project::*;
use crate::error::AppResult;
use crate::services::projects;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn project_list(state: State<'_, AppState>, include_archived: Option<bool>) -> AppResult<Vec<ProjectSummary>> {
    state.with_db(|db| projects::list(db, &state.projects_dir, include_archived.unwrap_or(false)))
}

#[tauri::command]
pub fn project_create(state: State<'_, AppState>, input: CreateProjectInput) -> AppResult<Project> {
    state.with_db(|db| projects::create(db, &state.projects_dir, input))
}

#[tauri::command]
pub fn project_get(state: State<'_, AppState>, id: String) -> AppResult<Project> {
    state.with_db(|db| projects::get(db, &id))
}

#[tauri::command]
pub fn project_update(state: State<'_, AppState>, input: UpdateProjectInput) -> AppResult<Project> {
    state.with_db(|db| projects::update(db, &state.projects_dir, input))
}

#[tauri::command]
pub fn project_set_archived(state: State<'_, AppState>, id: String, archived: bool) -> AppResult<()> {
    state.with_db(|db| projects::set_archived(db, &id, archived))
}

#[tauri::command]
pub fn project_open(state: State<'_, AppState>, id: String) -> AppResult<Project> {
    state.with_db(|db| {
        projects::mark_opened(db, &id)?;
        projects::get(db, &id)
    })
}

#[tauri::command]
pub fn project_delete(state: State<'_, AppState>, id: String, confirm_name: String) -> AppResult<()> {
    state.with_db(|db| projects::delete(db, &state.projects_dir, &id, &confirm_name))
}
