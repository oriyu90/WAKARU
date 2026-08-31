use crate::domain::{AppInfo, Job};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn app_get_info(state: State<'_, AppState>) -> AppResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_date: option_env!("WAKARU_BUILD_DATE").unwrap_or("dev").to_string(),
        data_dir: state.data_dir.display().to_string(),
        license: "MIT".to_string(),
    })
}

#[tauri::command]
pub fn app_open_data_dir(state: State<'_, AppState>, app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(state.data_dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::internal(format!("opener: {e}")))
}

#[tauri::command]
pub fn jobs_list(state: State<'_, AppState>, project_id: Option<String>) -> AppResult<Vec<Job>> {
    Ok(state.jobs.list(project_id.as_deref()))
}

#[tauri::command]
pub fn jobs_cancel(state: State<'_, AppState>, job_id: String) -> AppResult<()> {
    if state.jobs.cancel(&job_id) {
        Ok(())
    } else {
        Err(AppError::new(
            "JOB_NOT_FOUND",
            "error.job.notFound",
            format!("no active job {job_id}"),
        ))
    }
}

#[tauri::command]
pub fn db_health(state: State<'_, AppState>) -> AppResult<String> {
    state.with_db(|conn| {
        Ok(conn.query_row(
            "SELECT value FROM schema_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )?)
    })
}
