//! MCP client (docs/05 §6, AC-7-1/3/4/11/12). Only the **stdio** transport is
//! implemented for v0.2.0; Streamable HTTP is deferred (DECISIONS D-14 — its
//! `rmcp` feature pulls native-tls / openssl-sys, a cross-platform build and
//! `cargo deny` liability). Connections live in a process-global registry so a
//! Studio tool loop can reach them without threading a handle through every
//! call. A server that fails to connect or dies isolates to itself (AC-7-11).

use crate::domain::mcp::{McpServer, McpTool};
use crate::error::{AppError, AppResult};
use rmcp::model::CallToolRequestParams;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::TokioChildProcess;
use rmcp::ServiceExt;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::sync::Mutex;

const KEYRING_SERVICE: &str = "com.yukiorita.wakaru";
/// Env vars a stdio MCP server inherits (docs/05 §6.4 — no secrets by default).
const ENV_ALLOWLIST: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG"];
const STDERR_KEEP_LINES: usize = 200;

struct Conn {
    service: RunningService<RoleClient, ()>,
    slug: String,
    server_name: String,
    tools: Vec<ToolInfo>,
    stderr: Arc<StdMutex<Vec<String>>>,
}

#[derive(Clone)]
struct ToolInfo {
    name: String,
    description: Option<String>,
    schema: Value,
}

type Registry = Mutex<HashMap<String, Conn>>;
static CONNS: OnceLock<Registry> = OnceLock::new();
fn conns() -> &'static Registry {
    CONNS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `<slug>` half of a namespaced tool name — ascii alnum + `_` only (docs/05 §6.3).
pub fn slugify(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let trimmed = s.trim_matches('_').to_string();
    if trimmed.is_empty() { "server".into() } else { trimmed }
}

// ───────────────────────── persistence helpers ─────────────────────────

/// One `mcp_servers` row, decoded. `connected` / `last_error` are filled in by
/// the caller from the live registry.
pub fn list_servers(app_db: &Connection) -> AppResult<Vec<McpServer>> {
    let mut stmt = app_db.prepare(
        "SELECT id, name, transport, command, args, url, env, enabled FROM mcp_servers ORDER BY created_at",
    )?;
    let rows: Vec<McpServer> = stmt
        .query_map([], |r| {
            let args: String = r.get(4)?;
            let env: String = r.get(6)?;
            let env_keys = serde_json::from_str::<Map<String, Value>>(&env)
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default();
            Ok(McpServer {
                id: r.get(0)?,
                name: r.get(1)?,
                transport: r.get(2)?,
                command: r.get(3)?,
                args: serde_json::from_str(&args).unwrap_or_default(),
                url: r.get(5)?,
                env_keys,
                enabled: r.get::<_, i64>(7)? != 0,
                connected: false,
                last_error: None,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub fn get_server(app_db: &Connection, id: &str) -> AppResult<McpServerRow> {
    app_db
        .query_row(
            "SELECT id, name, transport, command, args, url, env FROM mcp_servers WHERE id = ?1",
            [id],
            |r| {
                Ok(McpServerRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    transport: r.get(2)?,
                    command: r.get(3)?,
                    args: serde_json::from_str::<Vec<String>>(&r.get::<_, String>(4)?).unwrap_or_default(),
                    url: r.get(5)?,
                    env: serde_json::from_str::<Map<String, Value>>(&r.get::<_, String>(6)?).unwrap_or_default(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("MCP_SERVER_NOT_FOUND", "error.mcp.serverNotFound", id))
}

pub struct McpServerRow {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: Map<String, Value>,
}

/// Merge the stored per-tool policies for one server into the map the Studio
/// loop consults, keyed by `<slug>__<tool>` (missing => `ask`).
pub fn policy_map(app_db: &Connection) -> AppResult<HashMap<String, String>> {
    let mut stmt = app_db.prepare(
        "SELECT s.name, p.tool_name, p.policy
         FROM mcp_tool_policies p JOIN mcp_servers s ON s.id = p.server_id",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows
        .into_iter()
        .map(|(name, tool, policy)| (format!("{}__{}", slugify(&name), tool), policy))
        .collect())
}

pub fn set_policy(app_db: &Connection, server_id: &str, tool: &str, policy: &str) -> AppResult<()> {
    if !matches!(policy, "ask" | "always_allow" | "deny") {
        return Err(AppError::new("MCP_BAD_POLICY", "error.mcp.badPolicy", policy));
    }
    app_db.execute(
        "INSERT INTO mcp_tool_policies (server_id, tool_name, policy)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(server_id, tool_name) DO UPDATE SET policy = excluded.policy",
        params![server_id, tool, policy],
    )?;
    Ok(())
}

// ───────────────────────── connection lifecycle ─────────────────────────

/// Connect (or reconnect) one stdio server and cache its tool list. HTTP is not
/// supported yet (D-14).
pub async fn connect(row: McpServerRow, policies: &HashMap<String, String>) -> AppResult<Vec<McpTool>> {
    if row.transport != "stdio" {
        return Err(AppError::new(
            "MCP_TRANSPORT_UNSUPPORTED",
            "error.mcp.transportUnsupported",
            "only stdio MCP servers are supported in this release",
        ));
    }
    let program = row
        .command
        .clone()
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(|| AppError::new("MCP_NO_COMMAND", "error.mcp.noCommand", "stdio server needs a command"))?;

    // Build a shell-free command with a scrubbed environment (docs/05 §6.4).
    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&row.args).env_clear();
    for key in ENV_ALLOWLIST {
        if let Ok(v) = std::env::var(key) {
            cmd.env(key, v);
        }
    }
    for (k, v) in &row.env {
        let Some(v) = v.as_str() else { continue };
        let resolved = if let Some(reference) = v.strip_prefix("keychain:") {
            keyring::Entry::new(KEYRING_SERVICE, &format!("mcp_env:{}:{reference}", row.id))
                .ok()
                .and_then(|e| e.get_password().ok())
                .unwrap_or_default()
        } else {
            v.to_string()
        };
        cmd.env(k, resolved);
    }

    let (proc, stderr) = TokioChildProcess::builder(cmd)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AppError::new("MCP_SPAWN_FAILED", "error.mcp.spawnFailed", e.to_string()))?;

    let stderr_buf = Arc::new(StdMutex::new(Vec::<String>::new()));
    if let Some(err) = stderr {
        let buf = stderr_buf.clone();
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut g = buf.lock().unwrap();
                g.push(line);
                let overflow = g.len().saturating_sub(STDERR_KEEP_LINES);
                if overflow > 0 {
                    g.drain(0..overflow);
                }
            }
        });
    }

    let service = ()
        .serve(proc)
        .await
        .map_err(|e| AppError::new("MCP_HANDSHAKE_FAILED", "error.mcp.handshakeFailed", e.to_string()))?;

    let raw = service
        .list_all_tools()
        .await
        .map_err(|e| AppError::new("MCP_LIST_TOOLS_FAILED", "error.mcp.listToolsFailed", e.to_string()))?;

    let slug = slugify(&row.name);
    let tools: Vec<ToolInfo> = raw
        .into_iter()
        .map(|t| ToolInfo {
            name: t.name.to_string(),
            description: t.description.map(|d| d.to_string()),
            schema: serde_json::to_value(&*t.input_schema).unwrap_or_else(|_| json!({ "type": "object" })),
        })
        .collect();

    let out: Vec<McpTool> = tools
        .iter()
        .map(|t| {
            let qualified = format!("{slug}__{}", t.name);
            McpTool {
                server_id: row.id.clone(),
                server_name: row.name.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                policy: policies.get(&qualified).cloned().unwrap_or_else(|| "ask".into()),
                qualified_name: qualified,
            }
        })
        .collect();

    // Replace any previous connection for this server id.
    let mut g = conns().lock().await;
    if let Some(old) = g.remove(&row.id) {
        let _ = old.service.cancel().await;
    }
    g.insert(
        row.id.clone(),
        Conn { service, slug, server_name: row.name.clone(), tools, stderr: stderr_buf },
    );
    Ok(out)
}

pub async fn disconnect(server_id: &str) {
    if let Some(conn) = conns().lock().await.remove(server_id) {
        let _ = conn.service.cancel().await;
    }
}

pub async fn stderr_lines(server_id: &str) -> Vec<String> {
    match conns().lock().await.get(server_id) {
        Some(c) => c.stderr.lock().unwrap().clone(),
        None => Vec::new(),
    }
}

pub async fn connected_ids() -> Vec<String> {
    conns().lock().await.keys().cloned().collect()
}

/// OpenAI `tools` entries for every connected server, namespaced (docs/05 §6.3).
pub async fn studio_tool_defs() -> Vec<Value> {
    let g = conns().lock().await;
    let mut out = Vec::new();
    for conn in g.values() {
        for t in &conn.tools {
            let mut schema = t.schema.clone();
            if !schema.is_object() {
                schema = json!({ "type": "object" });
            }
            out.push(json!({
                "type": "function",
                "function": {
                    "name": format!("{}__{}", conn.slug, t.name),
                    "description": t.description.clone().unwrap_or_else(|| format!("{} (via {})", t.name, conn.server_name)),
                    "parameters": schema,
                }
            }));
        }
    }
    out
}

/// Call `<slug>__<tool>` on whichever connected server owns `slug`. The returned
/// string is later stored as a `role: "tool"` message — the loop's system
/// prompt tells the model that tool output is data, not instructions (AC-7-12).
pub async fn call_tool(slug: &str, tool: &str, arguments: &str) -> AppResult<String> {
    let g = conns().lock().await;
    let conn = g
        .values()
        .find(|c| c.slug == slug)
        .ok_or_else(|| AppError::new("MCP_NOT_CONNECTED", "error.mcp.notConnected", slug))?;

    let args: Option<Map<String, Value>> = serde_json::from_str(arguments.trim())
        .ok()
        .or_else(|| Some(Map::new()));
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Some(a) = args {
        if !a.is_empty() {
            params = params.with_arguments(a);
        }
    }

    let res = conn
        .service
        .call_tool(params)
        .await
        .map_err(|e| AppError::new("MCP_CALL_FAILED", "error.mcp.callFailed", e.to_string()))?;

    // Flatten text content; a structured result falls back to its JSON.
    let mut text = String::new();
    for block in &res.content {
        if let Some(t) = block.as_text() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&t.text);
        }
    }
    if text.is_empty() {
        if let Some(sc) = &res.structured_content {
            text = sc.to_string();
        }
    }
    if res.is_error.unwrap_or(false) {
        return Ok(format!("ERROR (tool): {text}"));
    }
    Ok(if text.is_empty() { "(no content)".into() } else { text })
}

/// Drop every live connection (called on app shutdown — stdio children must not
/// be orphaned, docs/05 §6.1).
pub async fn shutdown_all() {
    let mut g = conns().lock().await;
    for (_, conn) in g.drain() {
        let _ = conn.service.cancel().await;
    }
}
