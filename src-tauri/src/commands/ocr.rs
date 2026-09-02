//! Viewer-driven OCR for scanned PDFs (P12/4). The frontend rasterises each
//! text-less page with PDF.js and hands the PNG here; the backend OCRs it,
//! folds the text into search, and (on finalize) builds `searchable.pdf`.

use crate::error::{AppError, AppResult};
use crate::services::{ocr, projects, sources};
use crate::state::AppState;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrPageInput {
    pub project_id: String,
    pub source_id: String,
    pub page: u32,
    /// Total page count (for the page header text).
    pub total: u32,
    /// base64 PNG of the rendered page (no data: prefix).
    pub png_base64: String,
}

/// OCR one rasterised page. Returns the recognised lines/word boxes so the
/// Viewer can draw its selectable text overlay.
#[tauri::command]
pub fn ocr_page(state: State<'_, AppState>, input: OcrPageInput) -> AppResult<ocr::OcrPage> {
    let png = base64::engine::general_purpose::STANDARD
        .decode(input.png_base64.trim())
        .map_err(|_| AppError::new("OCR_BAD_INPUT", "errors.ocr.image", "invalid page image"))?;
    if png.len() > 25 * 1024 * 1024 {
        return Err(AppError::new(
            "OCR_BAD_INPUT",
            "errors.ocr.image",
            "page image is too large",
        ));
    }
    let project_db = projects::open_db(&state.projects_dir, &input.project_id)?;
    let app_db = state
        .app_db
        .lock()
        .map_err(|_| AppError::internal("app_db mutex poisoned"))?;
    let name = sources::get(&project_db, &input.source_id)?.original_name;
    let project_dir = projects::project_dir(&state.projects_dir, &input.project_id);

    ocr::apply_pdf_page(
        &project_db,
        &app_db,
        &state.data_dir,
        &project_dir,
        &input.project_id,
        &input.source_id,
        &name,
        input.page,
        input.total,
        &png,
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrFinalizeInput {
    pub project_id: String,
    pub source_id: String,
    /// false when one or more pages failed.
    pub all_ok: bool,
}

/// Mark OCR complete and build `searchable.pdf`. Returns its project-relative
/// asset path when one was produced.
#[tauri::command]
pub fn ocr_finalize(
    state: State<'_, AppState>,
    input: OcrFinalizeInput,
) -> AppResult<Option<String>> {
    let project_db = projects::open_db(&state.projects_dir, &input.project_id)?;
    let project_dir = projects::project_dir(&state.projects_dir, &input.project_id);
    ocr::finalize_pdf(&project_db, &project_dir, &input.source_id, input.all_ok)
}
