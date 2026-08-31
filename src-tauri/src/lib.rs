//! WAKARU backend entry (docs/02 §2). Builds the Tauri app, opens `app.db`, runs
//! migrations, registers plugins and the IPC command surface.

mod commands;
mod domain;
mod error;
mod jobs;
mod logging;
mod paths;
mod state;
mod storage;

use jobs::JobRegistry;
use state::AppState;
use std::sync::Mutex;
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
            let _ = paths::projects_dir(handle)?;

            app.manage(AppState {
                app_db: Mutex::new(conn),
                jobs: JobRegistry::default(),
                data_dir,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running WAKARU");
}
