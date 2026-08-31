//! MCP server registry types (docs/05 §6, docs/02 §4.8). Servers are registered
//! by hand and connected on demand; a failed connection isolates to that server
//! (AC-7-11). Secret env values are keychain references, never plaintext (I-4).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    /// `"stdio"` (supported) or `"http"` (deferred — DECISIONS D-14).
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    /// Names of env vars this server sets (values are not exposed — a value may
    /// be a `keychain:<ref>`).
    pub env_keys: Vec<String>,
    pub enabled: bool,
    pub connected: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub server_id: String,
    pub server_name: String,
    /// The tool's own name, as the server reports it.
    pub name: String,
    pub description: Option<String>,
    /// `"<serverSlug>__<name>"` — what the model sees (docs/05 §6.3).
    pub qualified_name: String,
    /// `"ask"` | `"always_allow"` | `"deny"`.
    pub policy: String,
}

#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct McpUpsertInput {
    pub id: Option<String>,
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// `name -> value`. A value of `keychain:<ref>` is resolved from the OS
    /// keychain at connect time; anything else is stored as-is.
    #[serde(default)]
    #[ts(type = "Record<string, string>")]
    pub env: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct McpConnectResult {
    pub tools: Vec<McpTool>,
}
