use crate::domain::ai::*;
use crate::error::{AppError, AppResult};
use crate::services::ai;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn ai_list_profiles(state: State<'_, AppState>) -> AppResult<Vec<AiProfile>> {
    state.with_db(ai::profiles::list)
}

#[tauri::command]
pub fn ai_upsert_profile(state: State<'_, AppState>, input: AiProfileInput) -> AppResult<AiProfile> {
    state.with_db(|db| ai::profiles::upsert(db, input))
}

#[tauri::command]
pub fn ai_delete_profile(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.with_db(|db| ai::profiles::delete(db, &id))
}

#[tauri::command]
pub async fn ai_test_profile(state: State<'_, AppState>, id: String) -> AppResult<TestResult> {
    let path = state.app_db_path.clone();
    ai::test_profile(&path, &id).await
}

#[tauri::command]
pub fn ai_get_role_bindings(state: State<'_, AppState>) -> AppResult<RoleBindings> {
    state.with_db(ai::profiles::get_bindings)
}

#[tauri::command]
pub fn ai_set_role_binding(
    state: State<'_, AppState>,
    role: Role,
    profile_id: String,
    model: String,
    params: Option<serde_json::Value>,
) -> AppResult<()> {
    state.with_db(|db| {
        ai::profiles::set_binding(db, role, &profile_id, &model, params.unwrap_or(serde_json::json!({})))
    })
}

#[tauri::command]
pub fn ai_clear_role_binding(state: State<'_, AppState>, role: Role) -> AppResult<()> {
    state.with_db(|db| ai::profiles::clear_binding(db, role))
}

#[tauri::command]
pub fn ai_cancel_request(state: State<'_, AppState>, stream_id: String) -> AppResult<()> {
    if state.streams.cancel(&stream_id) {
        Ok(())
    } else {
        Err(AppError::new("STREAM_NOT_FOUND", "error.ai.streamNotFound", stream_id))
    }
}

/// Debug helper (docs/08 Phase 3: "check via the debug search-result view").
/// Streams a one-shot chat completion for the `chat` role and returns the
/// stream id; deltas arrive on `stream://*`.
#[tauri::command]
pub async fn ai_debug_chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    prompt: String,
) -> AppResult<String> {
    let resolved = state
        .with_db(|db| ai::profiles::resolve(db, Role::Chat))?
        .ok_or_else(|| AppError::new("AI_NOT_CONFIGURED", "error.ai.notConfigured", "no chat model set"))?;
    let (stream_id, token) = state.streams.start();
    let reg = state.streams.clone();
    let messages = serde_json::json!([{ "role": "user", "content": prompt }]);
    let sid = stream_id.clone();
    tauri::async_runtime::spawn(async move {
        ai::stream_chat(&app, &reg, resolved, messages, sid, token).await;
    });
    Ok(stream_id)
}
