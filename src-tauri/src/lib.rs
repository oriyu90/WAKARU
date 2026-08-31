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
            commands::viewer_get_tabs,
            commands::viewer_open_tab,
            commands::viewer_close_tab,
            commands::viewer_update_locator,
            commands::viewer_pin_tab,
            commands::viewer_reorder_tabs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WAKARU");
}

/// Serve a `wakaru-asset://` request from a project's sandboxed files.
fn asset_response(
    app: &tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<std::borrow::Cow<'static, [u8]>> {
    use tauri::http::{Response, StatusCode};
    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(std::borrow::Cow::Borrowed(&b""[..]))
            .unwrap()
    };

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
                .body(std::borrow::Cow::Borrowed(&b""[..]))
                .unwrap();
        }
    };
    match std::fs::read(&resolved) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", services::assets::content_type(&resolved))
            .header("Cache-Control", "no-cache")
            .body(std::borrow::Cow::Owned(bytes))
            .unwrap(),
        Err(_) => not_found(),
    }
}
