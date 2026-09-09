use crate::domain::viewer::*;
use crate::error::AppResult;
use crate::services::{assets, projects, viewer};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn viewer_get_tabs(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<ViewerTab>> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::get_tabs(&db)
}

#[tauri::command]
pub fn viewer_open_tab(state: State<'_, AppState>, input: OpenTabInput) -> AppResult<ViewerTab> {
    let db = projects::open_db(&state.projects_dir, &input.project_id)?;
    viewer::open_tab(&db, &input.source_id, input.locator)
}

#[tauri::command]
pub fn viewer_close_tab(
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::close_tab(&db, &tab_id)
}

#[tauri::command]
pub fn viewer_update_locator(
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
    locator: serde_json::Value,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::update_locator(&db, &tab_id, locator)
}

#[tauri::command]
pub fn viewer_pin_tab(
    state: State<'_, AppState>,
    project_id: String,
    tab_id: String,
    pinned: bool,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::pin_tab(&db, &tab_id, pinned)
}

#[tauri::command]
pub fn viewer_reorder_tabs(
    state: State<'_, AppState>,
    project_id: String,
    tab_ids: Vec<String>,
) -> AppResult<()> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::reorder_tabs(&db, &tab_ids)
}

#[tauri::command]
pub fn source_get_document(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
    ordinal: u32,
) -> AppResult<DocumentPayload> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::get_document(&db, &state.projects_dir, &project_id, &source_id, ordinal)
}

#[tauri::command]
pub fn source_detail(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
) -> AppResult<SourceDetail> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    viewer::source_detail(&db, &project_id, &source_id)
}

#[tauri::command]
pub fn source_asset_url(
    project_id: String,
    source_id: String,
    rel_path: String,
) -> AppResult<String> {
    Ok(assets::url(&project_id, &source_id, &rel_path))
}

/// File list + entry for a `website` source, for the Viewer's file tree.
#[tauri::command]
pub fn website_manifest(
    state: State<'_, AppState>,
    project_id: String,
    source_id: String,
) -> AppResult<crate::services::website::WebsiteManifest> {
    let db = projects::open_db(&state.projects_dir, &project_id)?;
    let rel: String = db.query_row(
        "SELECT rel_path FROM sources WHERE id = ?1 AND kind = 'website'",
        [&source_id],
        |r| r.get(0),
    )?;
    let entry = rel
        .strip_prefix(&format!("sources/{source_id}/"))
        .unwrap_or("index.html")
        .to_string();
    let site_root = projects::project_dir(&state.projects_dir, &project_id)
        .join("sources")
        .join(&source_id);
    crate::services::website::manifest(&site_root, &entry)
}
