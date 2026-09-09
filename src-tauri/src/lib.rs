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
            commands::source_reanalyze,
            commands::source_delete,
            commands::source_get_document,
            commands::source_detail,
            commands::source_asset_url,
            commands::ocr_page,
            commands::ocr_finalize,
            commands::viewer_get_tabs,
            commands::viewer_open_tab,
            commands::viewer_close_tab,
            commands::viewer_update_locator,
            commands::viewer_pin_tab,
            commands::viewer_reorder_tabs,
            commands::ai_list_profiles,
            commands::ai_upsert_profile,
            commands::ai_delete_profile,
            commands::ai_test_profile,
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
    match std::fs::read(&resolved) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", services::assets::content_type(&resolved))
            .header("Cache-Control", "no-cache")
            .header("Access-Control-Allow-Origin", "*")
            .body(std::borrow::Cow::Owned(bytes))
            .unwrap(),
        Err(_) => not_found(),
    }
}
