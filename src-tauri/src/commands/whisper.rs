use crate::domain::transcription::{DiskCheck, WhisperModel};
use crate::error::AppResult;
use crate::services::{settings, whisper};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    name: String,
    done: u64,
    total: u64,
}

#[tauri::command]
pub fn whisper_list_models(state: State<'_, AppState>) -> AppResult<Vec<WhisperModel>> {
    let data_dir = state.data_dir.clone();
    state.with_db(|db| {
        let selected = settings::get(db)?.transcription.whisper_model;
        whisper::list(db, &data_dir, &selected)
    })
}

#[tauri::command]
pub fn whisper_disk_check(state: State<'_, AppState>, name: String) -> AppResult<DiskCheck> {
    whisper::disk_check(&state.data_dir, &name)
}

#[tauri::command]
pub async fn whisper_download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> AppResult<()> {
    let app_db_path = state.app_db_path.clone();
    let data_dir = state.data_dir.clone();
    whisper::clear_cancel(&name);
    let name2 = name.clone();

    let res = tauri::async_runtime::spawn_blocking(move || {
        let db = crate::storage::open(&app_db_path)?;
        let mut last = 0u64;
        whisper::download(
            &db,
            &data_dir,
            &name2,
            &|| whisper::is_cancel_requested(&name2),
            &mut |done, total| {
                // Throttle events to ~1 MB steps.
                if done - last >= 1_048_576 || done == total {
                    last = done;
                    let _ = app.emit(
                        "whisper://download",
                        DownloadProgress { name: name2.clone(), done, total },
                    );
                }
            },
        )
    })
    .await
    .map_err(|e| crate::error::AppError::internal(format!("join: {e}")))?;

    whisper::clear_cancel(&name);
    res
}

#[tauri::command]
pub fn whisper_cancel_download(_state: State<'_, AppState>, name: String) -> AppResult<()> {
    whisper::request_cancel(&name);
    Ok(())
}

#[tauri::command]
pub fn whisper_delete_model(state: State<'_, AppState>, name: String) -> AppResult<()> {
    let data_dir = state.data_dir.clone();
    state.with_db(|db| whisper::delete(db, &data_dir, &name))
}

#[tauri::command]
pub fn whisper_select_model(state: State<'_, AppState>, name: String) -> AppResult<()> {
    state.with_db(|db| {
        settings::update(db, serde_json::json!({ "transcription": { "whisperModel": name } }))?;
        Ok(())
    })
}
