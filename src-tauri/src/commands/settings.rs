use crate::domain::settings::Settings;
use crate::error::AppResult;
use crate::services::settings;
use crate::state::AppState;
use tauri::{Emitter, State};

#[tauri::command]
pub fn app_get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.with_db(settings::get)
}

#[tauri::command]
pub fn app_update_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> AppResult<Settings> {
    let updated = state.with_db(|db| settings::update(db, patch))?;
    let _ = app.emit("settings://changed", &updated);
    Ok(updated)
}
