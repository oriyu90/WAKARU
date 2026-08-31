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
        ])
        .run(tauri::generate_context!())
        .expect("error while running WAKARU");
}
