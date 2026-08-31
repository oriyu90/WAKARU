use crate::domain::file_modifier::*;
use crate::error::AppResult;
use crate::services::file_modifier as fm;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn fm_path_exists(path: String) -> AppResult<bool> {
    Ok(fm::path_exists(&path))
}

#[tauri::command]
pub fn fm_image_preview(path: String) -> AppResult<String> {
    fm::image_preview(&path)
}

#[tauri::command]
pub fn fm_images_to_pdf(input: ImagesToPdfInput) -> AppResult<WrittenFile> {
    fm::images_to_pdf(&input)
}

#[tauri::command]
pub fn fm_save_text(input: SaveTextInput) -> AppResult<WrittenFile> {
    fm::save_text(&input)
}

#[tauri::command]
pub async fn fm_text_to_markdown(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: TextToMarkdownInput,
    ui_lang: String,
) -> AppResult<String> {
    let reg = state.streams.clone();
    let app_db_path = state.app_db_path.clone();
    fm::text_to_markdown(&app, reg, &app_db_path, input.text, ui_lang).await
}
