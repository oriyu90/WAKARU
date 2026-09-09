//! MCP client (docs/05 §6, AC-7-1/3/4/11/12). Supports stdio and the MCP
//! Streamable HTTP transport. Connections live in a process-global registry so a
//! Studio tool loop can reach them without threading a handle through every
//! call. A server that fails to connect or dies isolates to itself (AC-7-11).

use crate::domain::mcp::{McpServer, McpTool};
use crate::error::{AppError, AppResult};
use rmcp::model::{CallToolRequestParams, ContentBlock, ProtocolVersion, ResourceContents};
use rmcp::service::ClientLifecycleMode;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::TokioChildProcess;
use rmcp::ClientServiceExt;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::sync::Mutex;

const KEYRING_SERVICE: &str = "com.yukiorita.wakaru";
/// Env vars a stdio MCP server inherits (docs/05 §6.4 — no secrets by default).
const ENV_ALLOWLIST: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG"];
const STDERR_KEEP_LINES: usize = 200;
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(25);
const LIST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
const CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

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
    alias: String,
    description: Option<String>,
    schema: Value,
}

fn decode_tools(raw: Vec<rmcp::model::Tool>) -> Vec<ToolInfo> {
    raw.into_iter()
        .map(|tool| ToolInfo {
            name: tool.name.to_string(),
            alias: tool_alias(&tool.name),
            description: tool.description.map(|description| description.to_string()),
            schema: serde_json::to_value(&*tool.input_schema)
                .unwrap_or_else(|_| json!({ "type": "object" })),
        })
        .collect()
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
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = s.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "server".into()
    } else {
        trimmed
    }
}

/// Stable, collision-resistant namespace for a configured server. Display names
/// are not unique, but model-visible function names must be.
pub fn server_slug(name: &str, id: &str) -> String {
    let mut base = slugify(name);
    base.truncate(12);
    let suffix: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    if suffix.is_empty() {
        base
    } else {
        format!("{base}_{suffix}")
    }
}

/// OpenAI-compatible function-name alias for an MCP tool name. MCP names may
/// be longer or contain characters rejected by compatible chat endpoints.
pub fn tool_alias(name: &str) -> String {
    let mut base = slugify(name);
    base.truncate(24);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    format!("{base}_{:08x}", hasher.finish() as u32)
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
                    args: serde_json::from_str::<Vec<String>>(&r.get::<_, String>(4)?)
                        .unwrap_or_default(),
                    url: r.get(5)?,
                    env: serde_json::from_str::<Map<String, Value>>(&r.get::<_, String>(6)?)
                        .unwrap_or_default(),
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
        "SELECT s.id, s.name, p.tool_name, p.policy
         FROM mcp_tool_policies p JOIN mcp_servers s ON s.id = p.server_id",
    )?;
    let rows: Vec<(String, String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, name, tool, policy)| {
            (
                format!("{}__{}", server_slug(&name, &id), tool_alias(&tool)),
                policy,
            )
        })
        .collect())
}

pub fn set_policy(app_db: &Connection, server_id: &str, tool: &str, policy: &str) -> AppResult<()> {
    if !matches!(policy, "ask" | "always_allow" | "deny") {
        return Err(AppError::new(
            "MCP_BAD_POLICY",
            "error.mcp.badPolicy",
            policy,
        ));
    }
    app_db.execute(
        "INSERT INTO mcp_tool_policies (server_id, tool_name, policy)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(server_id, tool_name) DO UPDATE SET policy = excluded.policy",
        params![server_id, tool, policy],
    )?;
    Ok(())
}

// ───────────────────────── stdio command resolution ─────────────────────────
//
// A macOS app launched from Finder inherits only `PATH=/usr/bin:/bin:/usr/sbin:
// /sbin`, so a stdio MCP server registered as `npx` / `uvx` / `node` (the common
// case — e.g. a SearXNG web-search server) fails to spawn unless the user hunts
// down an absolute path. WAKARU widens the search to the well-known interpreter
// install locations, without a shell and without touching the secret-free env
// allowlist (docs/05 §6.4, D-29). This only *adds* candidate directories; a
// directory the user already had on `PATH` keeps its priority.

/// Well-known bin directories for language runtimes and tool managers that a
/// GUI-launched process does not get on `PATH`. Only ones that exist are used.
fn extra_bin_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
        "/opt/local/bin", // MacPorts
    ]
    .iter()
    .map(PathBuf::from)
    .collect();

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for rel in [
            ".local/bin",
            ".cargo/bin",
            ".bun/bin",
            ".deno/bin",
            ".volta/bin",
            "Library/pnpm",
            ".npm-global/bin",
            "n/bin",
        ] {
            dirs.push(home.join(rel));
        }
        // nvm / fnm keep versioned `node` under per-version dirs; include all,
        // sorted so the newest wins ties added later.
        for base in [
            home.join(".nvm/versions/node"),
            home.join(".local/share/fnm/node-versions"),
        ] {
            if let Ok(rd) = std::fs::read_dir(&base) {
                let mut found: Vec<PathBuf> = rd
                    .flatten()
                    .flat_map(|e| [e.path().join("bin"), e.path().join("installation/bin")])
                    .filter(|p| p.is_dir())
                    .collect();
                found.sort();
                dirs.extend(found);
            }
        }
    }
    dirs
}

/// `PATH` for a stdio MCP child: the inherited `PATH` first (user intent wins),
/// then [`extra_bin_dirs`], de-duplicated, existing directories only.
fn child_path() -> OsString {
    let mut seen = std::collections::HashSet::new();
    let mut ordered: Vec<PathBuf> = Vec::new();
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    for p in std::env::split_paths(&inherited)
        .chain(extra_bin_dirs())
        .filter(|p| !p.as_os_str().is_empty())
    {
        if p.is_dir() && seen.insert(p.clone()) {
            ordered.push(p);
        }
    }
    std::env::join_paths(&ordered).unwrap_or(inherited)
}

/// Resolve a bare command name against `search_path`. An absolute or
/// path-qualified command is returned unchanged; if nothing matches, the
/// original name is returned so `spawn` fails exactly as it did before.
fn resolve_program(program: &str, search_path: &OsString) -> OsString {
    if program.contains('/') || Path::new(program).is_absolute() {
        return program.into();
    }
    for dir in std::env::split_paths(search_path) {
        let candidate = dir.join(program);
        if !candidate.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            match std::fs::metadata(&candidate) {
                Ok(md) if md.permissions().mode() & 0o111 != 0 => {}
                _ => continue,
            }
        }
        return candidate.into_os_string();
    }
    program.into()
}

// ───────────────────────── connection lifecycle ─────────────────────────

/// Connect (or reconnect) one stdio server and cache its tool list. HTTP is not
/// supported yet (D-14).
pub async fn connect(
    row: McpServerRow,
    policies: &HashMap<String, String>,
) -> AppResult<Vec<McpTool>> {
    if row.transport == "http" {
        return connect_http(row, policies).await;
    }
    if row.transport != "stdio" {
        return Err(AppError::new(
            "MCP_TRANSPORT_UNSUPPORTED",
            "error.mcp.transportUnsupported",
            "supported MCP transports are stdio and http",
        ));
    }
    let program = row
        .command
        .clone()
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                "MCP_NO_COMMAND",
                "error.mcp.noCommand",
                "stdio server needs a command",
            )
        })?;

    // Build a shell-free command with a scrubbed environment (docs/05 §6.4).
    // The command name is resolved against a widened search path so `npx` /
    // `uvx` / `node` work without an absolute path (D-29); the child gets that
    // same widened `PATH` so a resolved `npx` can find its own `node`.
    let search_path = child_path();
    let program_path = resolve_program(&program, &search_path);
    let mut cmd = tokio::process::Command::new(&program_path);
    cmd.args(&row.args).env_clear();
    for key in ENV_ALLOWLIST {
        if *key == "PATH" {
            cmd.env("PATH", &search_path);
        } else if let Ok(v) = std::env::var(key) {
            cmd.env(key, v);
        }
    }
    let mut secret_values = Vec::new();
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
        if !resolved.is_empty() {
            secret_values.push(resolved.clone());
        }
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
            while let Ok(Some(mut line)) = lines.next_line().await {
                for secret in &secret_values {
                    line = line.replace(secret, "[REDACTED]");
                }
                let mut g = buf.lock().unwrap_or_else(|e| e.into_inner());
                g.push(line);
                let overflow = g.len().saturating_sub(STDERR_KEEP_LINES);
                if overflow > 0 {
                    g.drain(0..overflow);
                }
            }
        });
    }

    let service = tokio::time::timeout(
        CONNECT_TIMEOUT,
        ().serve_with_lifecycle(
            proc,
            ClientLifecycleMode::Auto {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                legacy_version: Some(ProtocolVersion::V_2025_11_25),
            },
        ),
    )
    .await
    .map_err(|_| {
        AppError::new(
            "MCP_HANDSHAKE_TIMEOUT",
            "error.mcp.handshakeFailed",
            "MCP handshake timed out",
        )
    })?
    .map_err(|e| {
        AppError::new(
            "MCP_HANDSHAKE_FAILED",
            "error.mcp.handshakeFailed",
            e.to_string(),
        )
    })?;

    let raw = tokio::time::timeout(LIST_TIMEOUT, service.list_all_tools())
        .await
        .map_err(|_| {
            AppError::new(
                "MCP_LIST_TOOLS_TIMEOUT",
                "error.mcp.listToolsFailed",
                "MCP tools/list timed out",
            )
        })?
        .map_err(|e| {
            AppError::new(
                "MCP_LIST_TOOLS_FAILED",
                "error.mcp.listToolsFailed",
                e.to_string(),
            )
        })?;

    let slug = server_slug(&row.name, &row.id);
    let tools = decode_tools(raw);

    let out: Vec<McpTool> = tools
        .iter()
        .map(|t| {
            let qualified = format!("{slug}__{}", t.alias);
            McpTool {
                server_id: row.id.clone(),
                server_name: row.name.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                policy: policies
                    .get(&qualified)
                    .cloned()
                    .unwrap_or_else(|| "ask".into()),
                qualified_name: qualified,
            }
        })
        .collect();

    // Replace any previous connection for this server id.
    let old = conns().lock().await.remove(&row.id);
    if let Some(mut old) = old {
        let _ = old.service.close_with_timeout(CLOSE_TIMEOUT).await;
    }
    conns().lock().await.insert(
        row.id.clone(),
        Conn {
            service,
            slug,
            server_name: row.name.clone(),
            tools,
            stderr: stderr_buf,
        },
    );
    Ok(out)
}

async fn connect_http(
    row: McpServerRow,
    policies: &HashMap<String, String>,
) -> AppResult<Vec<McpTool>> {
    use rmcp::transport::{
        streamable_http_client::StreamableHttpClientTransportConfig, StreamableHttpClientTransport,
    };

    // RMCP deliberately does not choose a process-wide rustls provider. WAKARU
    // uses ring consistently for its Rustls-backed HTTP clients.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let uri = row
        .url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::new("MCP_NO_URL", "error.mcp.noUrl", "HTTP server needs a URL"))?;
    validate_http_url(uri)?;

    let mut headers = HashMap::new();
    for (name, stored) in &row.env {
        let Some(stored) = stored.as_str() else {
            continue;
        };
        let value = resolve_secret(&row.id, stored);
        if value.is_empty() {
            continue;
        }
        let name = http::HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
            AppError::new(
                "MCP_HEADER_INVALID",
                "error.mcp.badHeader",
                error.to_string(),
            )
        })?;
        let value = http::HeaderValue::from_str(&value).map_err(|error| {
            AppError::new(
                "MCP_HEADER_INVALID",
                "error.mcp.badHeader",
                error.to_string(),
            )
        })?;
        headers.insert(name, value);
    }

    let config =
        StreamableHttpClientTransportConfig::with_uri(uri.to_string()).custom_headers(headers);
    let transport = StreamableHttpClientTransport::from_config(config);
    let service = tokio::time::timeout(
        CONNECT_TIMEOUT,
        ().serve_with_lifecycle(
            transport,
            ClientLifecycleMode::Auto {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                legacy_version: Some(ProtocolVersion::V_2025_11_25),
            },
        ),
    )
    .await
    .map_err(|_| {
        AppError::new(
            "MCP_HANDSHAKE_TIMEOUT",
            "error.mcp.handshakeFailed",
            "MCP handshake timed out",
        )
    })?
    .map_err(|error| {
        AppError::new(
            "MCP_HANDSHAKE_FAILED",
            "error.mcp.handshakeFailed",
            error.to_string(),
        )
    })?;

    let raw = tokio::time::timeout(LIST_TIMEOUT, service.list_all_tools())
        .await
        .map_err(|_| {
            AppError::new(
                "MCP_LIST_TOOLS_TIMEOUT",
                "error.mcp.listToolsFailed",
                "MCP tools/list timed out",
            )
        })?
        .map_err(|error| {
            AppError::new(
                "MCP_LIST_TOOLS_FAILED",
                "error.mcp.listToolsFailed",
                error.to_string(),
            )
        })?;
    let slug = server_slug(&row.name, &row.id);
    let tools = decode_tools(raw);
    let out = tools
        .iter()
        .map(|tool| {
            let qualified = format!("{slug}__{}", tool.alias);
            McpTool {
                server_id: row.id.clone(),
                server_name: row.name.clone(),
                name: tool.name.clone(),
                description: tool.description.clone(),
                policy: policies
                    .get(&qualified)
                    .cloned()
                    .unwrap_or_else(|| "ask".into()),
                qualified_name: qualified,
            }
        })
        .collect::<Vec<_>>();

    if let Some(mut old) = conns().lock().await.remove(&row.id) {
        let _ = old.service.close_with_timeout(CLOSE_TIMEOUT).await;
    }
    conns().lock().await.insert(
        row.id,
        Conn {
            service,
            slug,
            server_name: row.name,
            tools,
            stderr: Arc::new(StdMutex::new(Vec::new())),
        },
    );
    Ok(out)
}

fn resolve_secret(server_id: &str, stored: &str) -> String {
    stored
        .strip_prefix("keychain:")
        .and_then(|reference| {
            keyring::Entry::new(KEYRING_SERVICE, &format!("mcp_env:{server_id}:{reference}"))
                .ok()
                .and_then(|entry| entry.get_password().ok())
        })
        .unwrap_or_else(|| stored.to_string())
}

pub fn validate_http_url(raw: &str) -> AppResult<()> {
    let parsed = url::Url::parse(raw)
        .map_err(|error| AppError::new("MCP_URL_INVALID", "error.mcp.badUrl", error.to_string()))?;
    if parsed.username() != "" || parsed.password().is_some() || parsed.fragment().is_some() {
        return Err(AppError::new(
            "MCP_URL_INVALID",
            "error.mcp.badUrl",
            "credentials and fragments are not allowed in MCP URLs",
        ));
    }
    if parsed.scheme() == "https" {
        return Ok(());
    }
    if parsed.scheme() != "http" {
        return Err(AppError::new(
            "MCP_URL_INVALID",
            "error.mcp.badUrl",
            "MCP URL must use https, or http on a private network",
        ));
    }
    let private = match parsed.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        Some(url::Host::Ipv6(ip)) => {
            ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
        }
        Some(url::Host::Domain(host)) => {
            host.eq_ignore_ascii_case("localhost") || host.ends_with(".local")
        }
        None => false,
    };
    if private {
        Ok(())
    } else {
        Err(AppError::new(
            "MCP_INSECURE_REMOTE_URL",
            "error.mcp.insecureUrl",
            "plain HTTP is limited to localhost and private-network addresses",
        ))
    }
}

pub async fn disconnect(server_id: &str) {
    if let Some(mut conn) = conns().lock().await.remove(server_id) {
        let _ = conn.service.close_with_timeout(CLOSE_TIMEOUT).await;
    }
}

pub async fn stderr_lines(server_id: &str) -> Vec<String> {
    match conns().lock().await.get(server_id) {
        Some(c) => c.stderr.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        None => Vec::new(),
    }
}

pub async fn connected_ids() -> Vec<String> {
    conns()
        .lock()
        .await
        .iter()
        .filter(|(_, conn)| !conn.service.is_closed())
        .map(|(id, _)| id.clone())
        .collect()
}

/// OpenAI `tools` entries for every connected server, namespaced (docs/05 §6.3).
pub async fn studio_tool_defs() -> Vec<Value> {
    // Re-list before building the model tool catalog. RMCP applies modern cache
    // hints where available; legacy servers are simply queried again. A failed
    // refresh keeps the last known catalog and does not break Studio.
    let peers: Vec<(String, rmcp::Peer<RoleClient>)> = conns()
        .lock()
        .await
        .iter()
        .filter(|(_, conn)| !conn.service.is_closed())
        .map(|(id, conn)| (id.clone(), conn.service.peer().clone()))
        .collect();
    for (id, peer) in peers {
        if let Ok(Ok(raw)) = tokio::time::timeout(LIST_TIMEOUT, peer.list_all_tools()).await {
            if let Some(conn) = conns().lock().await.get_mut(&id) {
                conn.tools = decode_tools(raw);
            }
        }
    }

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
                    "name": format!("{}__{}", conn.slug, t.alias),
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
    let (peer, tool_name) = {
        let g = conns().lock().await;
        let conn = g
            .values()
            .find(|c| c.slug == slug)
            .ok_or_else(|| AppError::new("MCP_NOT_CONNECTED", "error.mcp.notConnected", slug))?;
        let name = conn
            .tools
            .iter()
            .find(|candidate| candidate.alias == tool)
            .map(|candidate| candidate.name.clone())
            .ok_or_else(|| AppError::new("MCP_TOOL_NOT_FOUND", "error.mcp.callFailed", tool))?;
        (conn.service.peer().clone(), name)
    };

    let args = if arguments.trim().is_empty() {
        Map::new()
    } else {
        serde_json::from_str::<Value>(arguments.trim())
            .map_err(|e| AppError::new("MCP_BAD_ARGUMENTS", "error.mcp.callFailed", e.to_string()))?
            .as_object()
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "MCP_BAD_ARGUMENTS",
                    "error.mcp.callFailed",
                    "tool arguments must be a JSON object",
                )
            })?
    };
    let mut params = CallToolRequestParams::new(tool_name);
    if !args.is_empty() {
        params = params.with_arguments(args);
    }

    let res = tokio::time::timeout(CALL_TIMEOUT, peer.call_tool(params))
        .await
        .map_err(|_| {
            AppError::new(
                "MCP_CALL_TIMEOUT",
                "error.mcp.callFailed",
                "MCP tool call timed out",
            )
        })?
        .map_err(|e| AppError::new("MCP_CALL_FAILED", "error.mcp.callFailed", e.to_string()))?;

    // Keep every standard MCP content kind. Binary payloads are represented by
    // metadata so a tool cannot flood the model context with base64 data.
    let mut text = String::new();
    for block in &res.content {
        let part = match block {
            ContentBlock::Text(t) => t.text.clone(),
            ContentBlock::Image(i) => format!(
                "[image: {}, {} base64 characters]",
                i.mime_type,
                i.data.len()
            ),
            ContentBlock::Audio(a) => format!(
                "[audio: {}, {} base64 characters]",
                a.mime_type,
                a.data.len()
            ),
            ContentBlock::Resource(r) => match &r.resource {
                ResourceContents::TextResourceContents { uri, text, .. } => {
                    format!("[resource: {uri}]\n{text}")
                }
                ResourceContents::BlobResourceContents {
                    uri,
                    blob,
                    mime_type,
                    ..
                } => format!(
                    "[resource: {uri}, {}, {} base64 characters]",
                    mime_type.as_deref().unwrap_or("application/octet-stream"),
                    blob.len()
                ),
                _ => "[resource: unsupported content kind]".into(),
            },
            ContentBlock::ResourceLink(r) => format!("[resource link: {} ({})]", r.name, r.uri),
            _ => "[unsupported MCP content block]".into(),
        };
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&part);
    }
    if let Some(sc) = &res.structured_content {
        if !text.is_empty() {
            text.push_str("\n[structured content]\n");
        }
        text.push_str(&sc.to_string());
    }
    if res.is_error.unwrap_or(false) {
        return Ok(format!("ERROR (tool): {text}"));
    }
    Ok(if text.is_empty() {
        "(no content)".into()
    } else {
        text
    })
}

/// Drop every live connection (called on app shutdown — stdio children must not
/// be orphaned, docs/05 §6.1).
pub async fn shutdown_all() {
    let drained: Vec<Conn> = conns().lock().await.drain().map(|(_, conn)| conn).collect();
    for mut conn in drained {
        let _ = conn.service.close_with_timeout(CLOSE_TIMEOUT).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_path_is_deduped_and_all_dirs_exist() {
        let joined = child_path();
        let dirs: Vec<_> = std::env::split_paths(&joined).collect();
        for d in &dirs {
            assert!(d.is_dir(), "child_path yielded a non-existent dir: {d:?}");
        }
        let unique: std::collections::HashSet<_> = dirs.iter().collect();
        assert_eq!(unique.len(), dirs.len(), "child_path has duplicate entries");
        // A POSIX system always has /bin or /usr/bin.
        assert!(dirs
            .iter()
            .any(|d| d == Path::new("/bin") || d == Path::new("/usr/bin")));
    }

    #[test]
    fn resolve_program_finds_a_bare_name_on_path() {
        let path = child_path();
        let resolved = resolve_program("sh", &path);
        let p = Path::new(&resolved);
        assert!(
            p.is_absolute(),
            "sh should resolve to an absolute path: {resolved:?}"
        );
        assert!(p.is_file());
        assert_eq!(p.file_name().unwrap(), "sh");
    }

    #[test]
    fn resolve_program_passes_through_qualified_and_unknown_names() {
        let path = child_path();
        // Absolute path: unchanged.
        assert_eq!(
            resolve_program("/usr/bin/env", &path),
            OsString::from("/usr/bin/env")
        );
        // Contains a slash: unchanged (relative command, caller's cwd).
        assert_eq!(
            resolve_program("./local-server", &path),
            OsString::from("./local-server")
        );
        // Not found anywhere: returned verbatim so spawn fails as before.
        assert_eq!(
            resolve_program("wakaru-no-such-binary-xyz", &path),
            OsString::from("wakaru-no-such-binary-xyz")
        );
    }

    #[test]
    fn server_namespaces_are_stable_and_unique() {
        assert_eq!(server_slug("My Server", "abc-123"), "my_server_abc123");
        assert_ne!(
            server_slug("duplicate", "11111111-a"),
            server_slug("duplicate", "22222222-b")
        );
        let alias = tool_alias("tool.with spaces/and-a-very-long-name-that-exceeds-limits");
        assert!(alias.len() <= 33);
        assert!(alias
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_'));
    }

    /// Manual interoperability check against the official MCP "everything"
    /// server. Ignored in normal CI because it downloads an npm package.
    #[tokio::test]
    #[ignore = "requires network access and npx"]
    async fn official_everything_server_lists_and_calls_tools() {
        let row = McpServerRow {
            id: "everything".into(),
            name: "Everything".into(),
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec![
                "-y".into(),
                "@modelcontextprotocol/server-everything".into(),
            ],
            url: None,
            env: Map::new(),
        };
        let tools = connect(row, &HashMap::new()).await.unwrap();
        assert!(tools.iter().any(|tool| tool.name == "echo"));
        let result = call_tool(
            &server_slug("Everything", "everything"),
            &tool_alias("echo"),
            r#"{"message":"WAKARU_MCP_OK"}"#,
        )
        .await
        .unwrap();
        assert!(result.contains("WAKARU_MCP_OK"));
        disconnect("everything").await;
    }
}
