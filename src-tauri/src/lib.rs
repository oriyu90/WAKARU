//! WAKARU backend entry (docs/02 §2). Builds the Tauri app, opens `app.db`, runs
//! migrations, registers plugins and the IPC command surface.

mod commands;
mod jobs;
mod logging;
mod paths;
mod state;

// Exposed for the integration tests in `tests/` (they can only see the public
// API of the lib crate). This is a private, unpublished app crate.
pub mod domain;
pub mod error;
pub mod services;
pub mod storage;

pub use domain::source::SourceKind;
pub use services::retrieval::keyword_search;

use jobs::JobRegistry;
use services::ai::StreamRegistry;
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .register_uri_scheme_protocol(services::assets::SCHEME, |ctx, request| {
            asset_response(ctx.app_handle(), request)
        })
        .setup(|app| {
            let handle = app.handle();

            let logs = paths::logs_dir(handle)?;
            logging::init(&logs);

            let db_path = paths::app_db_path(handle)?;
            tracing::info!(path = %db_path.display(), "opening app.db");
            let conn = storage::open_app_db(&db_path)?;
            let data_dir = paths::data_dir(handle)?;
            let projects_dir = paths::projects_dir(handle)?;

            // Startup GC: drop project folders with no matching row (docs/03 §8).
            if let Err(e) = services::projects::gc_orphans(&conn, &projects_dir) {
                tracing::warn!(error = %e, "orphan GC failed");
            }

            app.manage(AppState {
                app_db: Mutex::new(conn),
                app_db_path: db_path,
                jobs: Arc::new(JobRegistry::default()),
                streams: Arc::new(StreamRegistry::default()),
                data_dir,
                projects_dir,
            });

            tracing::info!(version = env!("CARGO_PKG_VERSION"), "WAKARU backend ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_get_info,
            commands::app_open_data_dir,
            commands::jobs_list,
            commands::jobs_cancel,
            commands::db_health,
            commands::project_list,
            commands::project_create,
            commands::project_get,
            commands::project_update,
            commands::project_set_archived,
            commands::project_open,
            commands::project_delete,
            commands::source_list,
            commands::source_get,
            commands::source_add_files,
            commands::source_add_url,
            commands::source_add_folder,
            commands::source_reanalyze,
            commands::source_delete,
            commands::source_get_document,
            commands::source_detail,
            commands::source_asset_url,
            commands::website_manifest,
            commands::ocr_page,
            commands::ocr_finalize,
            commands::viewer_get_tabs,
            commands::viewer_open_tab,
            commands::viewer_close_tab,
            commands::viewer_update_locator,
            commands::viewer_pin_tab,
            commands::viewer_reorder_tabs,
            commands::notes_list,
            commands::notes_create,
            commands::notes_update,
            commands::notes_delete,
            commands::notes_restore,
            commands::visual_create,
            commands::visual_get,
            commands::visual_list_for_message,
            commands::source_read_window,
            commands::ai_list_profiles,
            commands::ai_upsert_profile,
            commands::ai_delete_profile,
            commands::ai_test_profile,
            commands::ai_list_models,
            commands::ai_get_role_bindings,
            commands::ai_set_role_binding,
            commands::ai_clear_role_binding,
            commands::ai_cancel_request,
            commands::ai_debug_chat,
            commands::search_query,
            commands::app_get_settings,
            commands::app_update_settings,
            commands::fm_path_exists,
            commands::fm_image_preview,
            commands::fm_images_to_pdf,
            commands::fm_save_text,
            commands::fm_text_to_markdown,
            commands::project_export_estimates,
            commands::project_export,
            commands::project_import,
            commands::illustrator_get_or_create_thread,
            commands::illustrator_generate,
            commands::illustrator_ask,
            commands::illustrator_cancel,
            commands::illustrator_import_to_studio,
            commands::illustrator_generate_visual,
            commands::studio_list_tabs,
            commands::studio_get_tab,
            commands::studio_create_tab,
            commands::studio_rename_tab,
            commands::studio_close_tab,
            commands::studio_reorder_tabs,
            commands::studio_send,
            commands::studio_cancel,
            commands::studio_resolve_tool,
            commands::studio_list_artifacts,
            commands::studio_import_files,
            commands::studio_import_artifact_as_source,
            commands::studio_download_artifact,
            commands::mcp_list_servers,
            commands::mcp_upsert_server,
            commands::mcp_delete_server,
            commands::mcp_connect,
            commands::mcp_disconnect,
            commands::mcp_set_tool_policy,
            commands::mcp_server_stderr,
            commands::whisper_list_models,
            commands::whisper_disk_check,
            commands::whisper_download_model,
            commands::whisper_cancel_download,
            commands::whisper_delete_model,
            commands::whisper_select_model,
        ])
        .build(tauri::generate_context!())
        .expect("error while building WAKARU")
        .run(|_app, event| {
            // Never orphan a stdio MCP child (docs/05 §6.1).
            if let tauri::RunEvent::Exit = event {
                tauri::async_runtime::block_on(services::mcp::shutdown_all());
            }
        });
}

/// Serve a `wakaru-asset://` request from a project's sandboxed files.
fn asset_response(
    app: &tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<std::borrow::Cow<'static, [u8]>> {
    use tauri::http::{Method, Response, StatusCode};
    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Access-Control-Allow-Origin", "*")
            .body(std::borrow::Cow::Borrowed(&b""[..]))
            .unwrap()
    };

    // WebKit treats `fetch(wakaru-asset://...)` as a cross-origin request even
    // though the protocol is registered only inside this app. Text, CSV and
    // transcript previews use fetch, while images/media load the same URLs as
    // element sources. Return CORS headers consistently so both paths work in
    // the signed production WebView.
    if request.method() == Method::OPTIONS {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, OPTIONS")
            .body(std::borrow::Cow::Borrowed(&b""[..]))
            .unwrap();
    }

    let Some(state) = app.try_state::<AppState>() else {
        return not_found();
    };
    // request.uri() -> wakaru-asset://localhost/<pid>/<sid>/<rel...>
    let path = request.uri().path().to_string();
    let resolved = match services::assets::resolve(&state.projects_dir, &path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(path, code = %e.code, "asset request denied");
            return Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Access-Control-Allow-Origin", "*")
                .body(std::borrow::Cow::Borrowed(&b""[..]))
                .unwrap();
        }
    };
    match std::fs::metadata(&resolved) {
        Ok(meta) if meta.is_file() => {
            let total = meta.len();
            let content_type = services::assets::content_type(&resolved).to_string();
            // Single-range support (plan §3.2): `HEAD` + one `Range` → 206 with
            // `Content-Range`; anything else → 416. One request is capped so a
            // 1 GiB original is never handed to the WebView at once.
            if *request.method() == Method::HEAD {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", content_type)
                    .header("Content-Length", total.to_string())
                    .header("Accept-Ranges", "bytes")
                    .header("Cache-Control", "no-cache")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(std::borrow::Cow::Borrowed(&b""[..]))
                    .unwrap();
            }
            if let Some(range_header) = request.headers().get("range").and_then(|v| v.to_str().ok())
            {
                return range_response(&resolved, &content_type, total, range_header);
            }
            // Unbounded GET on a huge original would repeat the old full-read
            // path; refuse it and force the UI onto the windowed/range path.
            const UNBOUNDED_GET_MAX: u64 = 512 * 1024 * 1024;
            if total > UNBOUNDED_GET_MAX {
                return Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(std::borrow::Cow::Borrowed(&b""[..]))
                    .unwrap();
            }
            match std::fs::read(&resolved) {
                Ok(bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", content_type)
                    .header("Content-Length", bytes.len().to_string())
                    .header("Accept-Ranges", "bytes")
                    .header("Cache-Control", "no-cache")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(std::borrow::Cow::Owned(bytes))
                    .unwrap(),
                Err(_) => not_found(),
            }
        }
        _ => not_found(),
    }
}

/// Single-range `206` responder. `RANGE_SINGLE_MAX` bounds one response so
/// large PDFs page in via `rangeChunkSize` instead of a full copy.
fn range_response(
    path: &std::path::Path,
    content_type: &str,
    total: u64,
    header: &str,
) -> tauri::http::Response<std::borrow::Cow<'static, [u8]>> {
    use tauri::http::{Response, StatusCode};
    const RANGE_SINGLE_MAX: u64 = 32 * 1024 * 1024;
    let denied = || {
        Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header("Content-Range", format!("bytes */{total}"))
            .header("Accept-Ranges", "bytes")
            .header("Access-Control-Allow-Origin", "*")
            .body(std::borrow::Cow::Borrowed(&b""[..]))
            .unwrap()
    };
    let spec = header.trim().strip_prefix("bytes=").unwrap_or("").trim();
    if spec.is_empty() || spec.contains(',') {
        return denied();
    }
    let (start, end): (u64, u64) = if let Some(suffix) = spec.strip_prefix('-') {
        let n: u64 = suffix.parse().unwrap_or(0);
        if n == 0 || total == 0 {
            return denied();
        }
        let n = n.min(total);
        (total - n, total - 1)
    } else {
        let mut it = spec.splitn(2, '-');
        let s: u64 = it.next().unwrap_or("").parse().unwrap_or(u64::MAX);
        let e_opt = it.next().unwrap_or("");
        if s == u64::MAX {
            return denied();
        }
        let e: u64 = if e_opt.is_empty() {
            total.saturating_sub(1)
        } else {
            e_opt.parse().unwrap_or(u64::MAX)
        };
        if s >= total || e < s {
            return denied();
        }
        (s, e.min(total - 1))
    };
    let mut len = end - start + 1;
    if len > RANGE_SINGLE_MAX {
        len = RANGE_SINGLE_MAX;
    }
    use std::io::{Read, Seek, SeekFrom};
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Access-Control-Allow-Origin", "*")
                .body(std::borrow::Cow::Borrowed(&b""[..]))
                .unwrap();
        }
    };
    if f.seek(SeekFrom::Start(start)).is_err() {
        return denied();
    }
    let mut buf = vec![0u8; len as usize];
    let mut read = 0usize;
    while read < buf.len() {
        match f.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(k) => read += k,
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(std::borrow::Cow::Borrowed(&b""[..]))
                    .unwrap();
            }
        }
    }
    buf.truncate(read);
    let end = start + (read as u64).saturating_sub(1);
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("Content-Type", content_type)
        .header("Content-Length", read.to_string())
        .header("Content-Range", format!("bytes {start}-{end}/{total}"))
        .header("Accept-Ranges", "bytes")
        .header("Cache-Control", "no-cache")
        .header("Access-Control-Allow-Origin", "*")
        .body(std::borrow::Cow::Owned(buf))
        .unwrap()
}
