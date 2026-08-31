use crate::domain::search::{SearchQuery, SearchResults};
use crate::error::AppResult;
use crate::services::search;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn search_query(
    state: State<'_, AppState>,
    query: SearchQuery,
) -> AppResult<SearchResults> {
    let app_db_path = state.app_db_path.clone();
    let projects_dir = state.projects_dir.clone();
    search::query(&app_db_path, &projects_dir, query).await
}
