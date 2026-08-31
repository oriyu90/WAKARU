use crate::domain::mcp::*;
use crate::error::{AppError, AppResult};
use crate::services::mcp;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

const KEYRING_SERVICE: &str = "com.yukiorita.wakaru";

#[tauri::command]
pub async fn mcp_list_servers(state: State<'_, AppState>) -> AppResult<Vec<McpServer>> {
    let connected = mcp::connected_ids().await;
    let mut rows = state.with_db(mcp::list_servers)?;
    for r in &mut rows {
        r.connected = connected.contains(&r.id);
    }
    Ok(rows)
}

#[tauri::command]
pub fn mcp_upsert_server(
    state: State<'_, AppState>,
    input: McpUpsertInput,
) -> AppResult<McpServer> {
    if input.name.trim().is_empty() {
        return Err(AppError::new(
            "MCP_NAME_REQUIRED",
            "error.mcp.nameRequired",
            "server name is required",
        ));
    }
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());

    // Every env value is treated as a secret: the real value goes to the OS
    // keychain, the row stores only `keychain:<KEY>` (I-4).
    let mut env_refs = serde_json::Map::new();
    if let Some(obj) = input.env.as_object() {
        for (k, v) in obj {
            let Some(val) = v.as_str() else { continue };
            let stored = if let Some(reference) = val.strip_prefix("keychain:") {
                format!("keychain:{reference}")
            } else {
                if let Ok(entry) =
                    keyring::Entry::new(KEYRING_SERVICE, &format!("mcp_env:{id}:{k}"))
                {
                    let _ = entry.set_password(val);
                }
                format!("keychain:{k}")
            };
            env_refs.insert(k.clone(), serde_json::Value::String(stored));
        }
    }

    state.with_db(|db| {
        db.execute(
            "INSERT INTO mcp_servers (id, name, transport, command, args, url, env, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name, transport = excluded.transport, command = excluded.command,
               args = excluded.args, url = excluded.url, env = excluded.env",
            params![
                id,
                input.name.trim(),
                input.transport,
                input.command,
                serde_json::to_string(&input.args).unwrap_or_else(|_| "[]".into()),
                input.url,
                serde_json::Value::Object(env_refs).to_string(),
                crate::storage::migrate::now_iso8601(),
            ],
        )?;
        let mut rows = mcp::list_servers(db)?;
        rows.retain(|r| r.id == id);
        rows.into_iter()
            .next()
            .ok_or_else(|| AppError::internal("server vanished after upsert"))
    })
}

#[tauri::command]
pub async fn mcp_delete_server(state: State<'_, AppState>, id: String) -> AppResult<()> {
    mcp::disconnect(&id).await;
    state.with_db(|db| {
        // Best-effort keychain cleanup for this server's env secrets.
        if let Ok(row) = mcp::get_server(db, &id) {
            for k in row.env.keys() {
                if let Ok(entry) =
                    keyring::Entry::new(KEYRING_SERVICE, &format!("mcp_env:{id}:{k}"))
                {
                    let _ = entry.delete_credential();
                }
            }
        }
        db.execute("DELETE FROM mcp_servers WHERE id = ?1", [&id])?;
        Ok(())
    })
}

#[tauri::command]
pub async fn mcp_connect(state: State<'_, AppState>, id: String) -> AppResult<McpConnectResult> {
    let (row, policies) =
        state.with_db(|db| Ok((mcp::get_server(db, &id)?, mcp::policy_map(db)?)))?;
    let tools = mcp::connect(row, &policies).await?;
    Ok(McpConnectResult { tools })
}

#[tauri::command]
pub async fn mcp_disconnect(_state: State<'_, AppState>, id: String) -> AppResult<()> {
    mcp::disconnect(&id).await;
    Ok(())
}

#[tauri::command]
pub fn mcp_set_tool_policy(
    state: State<'_, AppState>,
    server_id: String,
    tool_name: String,
    policy: String,
) -> AppResult<()> {
    state.with_db(|db| mcp::set_policy(db, &server_id, &tool_name, &policy))
}

#[tauri::command]
pub async fn mcp_server_stderr(_state: State<'_, AppState>, id: String) -> AppResult<Vec<String>> {
    Ok(mcp::stderr_lines(&id).await)
}
