use crate::domain::source::Source;
use crate::domain::studio::*;
use crate::error::{AppError, AppResult};
use crate::services::{projects, sources, studio};
use crate::state::AppState;
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn studio_list_tabs(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<StudioTab>> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::list_tabs(&db)
}

#[tauri::command]
pub fn studio_get_tab(
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
) -> AppResult<StudioTab> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::get_tab(&db, &tab_id)
}

#[tauri::command]
pub fn studio_create_tab(
    state: State<'_, AppState>,
    project_id: String,
    title: Option<String>,
) -> AppResult<StudioTab> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::create_tab(&db, title)
}

#[tauri::command]
pub fn studio_rename_tab(
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
    title: String,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::rename_tab(&db, &tab_id, &title)
}

#[tauri::command]
pub fn studio_close_tab(
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::close_tab(&db, &tab_id)
}

#[tauri::command]
pub fn studio_reorder_tabs(
    state: State<'_, AppState>,
    project_id: String,
    ordered_ids: Vec<String>,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::reorder_tabs(&db, &ordered_ids)
}

#[tauri::command]
pub async fn studio_send(
    app: AppHandle,
    state: State<'_, AppState>,
    input: StudioSendInput,
    ui_lang: String,
) -> AppResult<StudioSendResult> {
    let reg = state.streams.clone();
    let app_db_path = state.app_db_path.clone();
    let projects_dir = state.projects_dir.clone();
    studio::send_streaming(&app, &reg, &app_db_path, &projects_dir, input, ui_lang).await
}

#[tauri::command]
pub fn studio_cancel(state: State<'_, AppState>, tab_id: String) -> AppResult<()> {
    state.streams.cancel(&format!("studio:{tab_id}"));
    Ok(())
}

#[tauri::command]
pub async fn studio_resolve_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
    approved: bool,
    ui_lang: String,
) -> AppResult<StudioSendResult> {
    let reg = state.streams.clone();
    let app_db_path = state.app_db_path.clone();
    let projects_dir = state.projects_dir.clone();
    studio::resolve_tool_streaming(
        &app,
        &reg,
        &app_db_path,
        &projects_dir,
        studio::ResolveToolRequest {
            project_id,
            tab_id,
            approved,
            ui_lang,
        },
    )
    .await
}

#[tauri::command]
pub fn studio_list_artifacts(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<Artifact>> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::list_artifacts(&db)
}

#[tauri::command]
pub fn studio_import_artifact_as_source(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    artifact_id: String,
) -> AppResult<Source> {
    let abs = {
        let db = projects::open_db(&state.projects_dir, &project_id)?;
        studio::artifact_abs_path(&state.projects_dir, &project_id, &db, &artifact_id)?
    };
    let created = sources::add_files(
        &app,
        &state.app_db_path,
        &state.projects_dir,
        state.jobs.clone(),
        &project_id,
        vec![abs.to_string_lossy().to_string()],
    )?;
    let source = created.into_iter().next().ok_or_else(|| {
        AppError::new(
            "STUDIO_IMPORT_FAILED",
            "error.studio.importFailed",
            "nothing imported",
        )
    })?;
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    studio::mark_artifact_imported(&db, &artifact_id, &source.id)?;
    Ok(source)
}

#[tauri::command]
pub fn studio_download_artifact(
    state: State<'_, AppState>,
    project_id: String,
    artifact_id: String,
    dest_dir: String,
) -> AppResult<String> {
    let abs = {
        let db = projects::open_db(&state.projects_dir, &project_id)?;
        studio::artifact_abs_path(&state.projects_dir, &project_id, &db, &artifact_id)?
    };
    let name = abs
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "artifact".into());
    let dest = Path::new(&dest_dir).join(name);
    std::fs::copy(&abs, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}
