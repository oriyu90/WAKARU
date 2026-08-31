//! Studio (docs/06 §6, FR-T1..T7, AC-6-*). Free chat tabs over a project's
//! sources, with a small set of built-in tools and a `workspace/` the model can
//! write into. The agentic loop is request/response, not token-streamed
//! (DECISIONS D-13): `studio_send` runs the whole loop and returns a summary;
//! `studio_resolve_tool` resumes a loop that paused for a `write_file` approval
//! or hit the 10-round cap.

use crate::domain::ai::Role;
use crate::domain::illustrator::{ChatMessage, Citation};
use crate::domain::studio::*;
use crate::error::{AppError, AppResult};
use crate::services::ai::client::AiClient;
use crate::services::ai::profiles::{self, ResolvedRole};
use crate::services::illustrator::{build_rag_block, resolve_citations};
use crate::services::{projects, retrieval};
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod prompts {
    pub const EN: &str = include_str!("ai/prompts/studio.en.md");
    pub const JA: &str = include_str!("ai/prompts/studio.ja.md");
    pub const ZH: &str = include_str!("ai/prompts/studio.zh-Hans.md");

    pub fn studio(lang: &str) -> &'static str {
        match lang {
            "ja" => JA,
            "zh-Hans" | "zh" => ZH,
            _ => EN,
        }
    }
}

/// Tool round-trips one `studio_send` (or one "続行") will run before it stops
/// and asks the reader to continue (AC-6-8).
const MAX_TOOL_ROUNDS: u32 = 10;
/// Context budget for the assembled message array, in characters (~1 token ≈ 4
/// chars). Older turns above this are folded into a summary; the latest question
/// is never dropped (AC-6-9).
const BUDGET_CHARS: usize = 48_000;
const RAG_TOP_K: usize = 12;

/// Built-in tools that never touch anything outside the project DB / workspace
/// and so run without asking (docs/06 §6). `write_file` is the one exception.
const READ_ONLY_TOOLS: &[&str] = &[
    "search_sources",
    "read_document",
    "list_sources",
    "list_tabs",
    "read_tab",
    "read_file",
    "list_files",
];

fn requires_approval(tool: &str) -> bool {
    !READ_ONLY_TOOLS.contains(&tool)
}

// ───────────────────────── tab CRUD (FR-T1/T2) ─────────────────────────

pub fn list_tabs(db: &Connection) -> AppResult<Vec<StudioTab>> {
    let mut stmt = db.prepare(
        "SELECT id, thread_id, title, ordinal, scope FROM studio_tabs ORDER BY ordinal, created_at",
    )?;
    let rows: Vec<(String, String, String, u32, String)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? as u32, r.get(4)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    let mut out = Vec::with_capacity(rows.len());
    for (id, thread_id, title, ordinal, scope) in rows {
        out.push(StudioTab {
            messages: load_messages(db, &thread_id)?,
            id,
            thread_id,
            title,
            ordinal,
            scope,
        });
    }
    Ok(out)
}

pub fn create_tab(db: &Connection, title: Option<String>) -> AppResult<StudioTab> {
    let now = now_iso8601();
    let thread_id = Uuid::now_v7().to_string();
    let tab_id = Uuid::now_v7().to_string();
    let ord: i64 = db
        .query_row("SELECT COALESCE(MAX(ordinal), 0) + 1 FROM studio_tabs", [], |r| r.get(0))
        .unwrap_or(1);
    let title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("Chat {ord}"));

    db.execute(
        "INSERT INTO threads (id, scope, title, created_at, updated_at) VALUES (?1, 'studio', ?2, ?3, ?3)",
        params![thread_id, title, now],
    )?;
    db.execute(
        "INSERT INTO studio_tabs (id, thread_id, title, ordinal, scope, created_at)
         VALUES (?1, ?2, ?3, ?4, 'project', ?5)",
        params![tab_id, thread_id, title, ord, now],
    )?;

    Ok(StudioTab {
        id: tab_id,
        thread_id,
        title,
        ordinal: ord as u32,
        scope: "project".into(),
        messages: Vec::new(),
    })
}

pub fn rename_tab(db: &Connection, tab_id: &str, title: &str) -> AppResult<()> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::new(
            "STUDIO_TITLE_REQUIRED",
            "error.studio.titleRequired",
            "tab title is required",
        ));
    }
    let thread_id = tab_thread(db, tab_id)?;
    db.execute("UPDATE studio_tabs SET title = ?2 WHERE id = ?1", params![tab_id, title])?;
    db.execute("UPDATE threads SET title = ?2 WHERE id = ?1", params![thread_id, title])?;
    Ok(())
}

/// Closing a tab drops its conversation. `workspace/` files and their
/// `artifacts` rows survive — the FK is `ON DELETE SET NULL` (AC-6-10).
pub fn close_tab(db: &Connection, tab_id: &str) -> AppResult<()> {
    let thread_id = tab_thread(db, tab_id)?;
    db.execute("DELETE FROM studio_tabs WHERE id = ?1", [tab_id])?;
    db.execute("DELETE FROM threads WHERE id = ?1", [&thread_id])?;
    Ok(())
}

pub fn reorder_tabs(db: &Connection, ordered_ids: &[String]) -> AppResult<()> {
    for (i, id) in ordered_ids.iter().enumerate() {
        db.execute(
            "UPDATE studio_tabs SET ordinal = ?2 WHERE id = ?1",
            params![id, i as i64],
        )?;
    }
    Ok(())
}

fn tab_thread(db: &Connection, tab_id: &str) -> AppResult<String> {
    db.query_row("SELECT thread_id FROM studio_tabs WHERE id = ?1", [tab_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::new("STUDIO_TAB_NOT_FOUND", "error.studio.tabNotFound", tab_id))
}

fn load_messages(db: &Connection, thread_id: &str) -> AppResult<Vec<ChatMessage>> {
    let mut stmt = db.prepare(
        "SELECT id, role, content, citations, model, status, created_at, tool_calls
         FROM messages WHERE thread_id = ?1 ORDER BY created_at, id",
    )?;
    let out: Vec<ChatMessage> = stmt
        .query_map([thread_id], |r| {
            Ok(ChatMessage {
                id: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                citations: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or(json!([])),
                model: r.get(4)?,
                status: r.get(5)?,
                created_at: r.get(6)?,
                tool_calls: r
                    .get::<_, Option<String>>(7)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(out)
}

// ───────────────────────── artifacts (FR-T7) ─────────────────────────

pub fn list_artifacts(db: &Connection) -> AppResult<Vec<Artifact>> {
    let mut stmt = db.prepare(
        "SELECT id, thread_id, rel_path, bytes, mime, imported_source_id, created_at
         FROM artifacts ORDER BY created_at DESC",
    )?;
    let out = stmt
        .query_map([], |r| {
            Ok(Artifact {
                id: r.get(0)?,
                thread_id: r.get(1)?,
                rel_path: r.get(2)?,
                bytes: r.get::<_, i64>(3)?.max(0) as u64,
                mime: r.get(4)?,
                imported_source_id: r.get(5)?,
                created_at: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(out)
}

pub fn artifact_abs_path(root: &Path, project_id: &str, db: &Connection, artifact_id: &str) -> AppResult<PathBuf> {
    let rel: String = db
        .query_row("SELECT rel_path FROM artifacts WHERE id = ?1", [artifact_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::new("ARTIFACT_NOT_FOUND", "error.studio.artifactNotFound", artifact_id))?;
    Ok(projects::project_dir(root, project_id).join(rel))
}

pub fn mark_artifact_imported(db: &Connection, artifact_id: &str, source_id: &str) -> AppResult<()> {
    db.execute(
        "UPDATE artifacts SET imported_source_id = ?2 WHERE id = ?1",
        params![artifact_id, source_id],
    )?;
    Ok(())
}

// ───────────────────────── workspace path guard ─────────────────────────

/// Resolve a model-supplied relative path inside a tab's `workspace/`. Rejects
/// absolute paths, `..`, `~` and anything whose existing parent resolves outside
/// the workspace (symlink escape). The full sandbox lands in P7; this is the
/// minimal guard Studio needs now.
pub fn resolve_in_workspace(workspace: &Path, rel: &str) -> AppResult<PathBuf> {
    let rel = rel.trim();
    let deny = |msg: &str| AppError::new("STUDIO_PATH_DENIED", "error.studio.pathDenied", msg);
    if rel.is_empty() {
        return Err(deny("empty path"));
    }
    if rel.starts_with('~') {
        return Err(deny("home-relative paths are not allowed"));
    }
    let p = Path::new(rel);
    if p.is_absolute() || rel.starts_with('/') || rel.starts_with('\\') {
        return Err(deny("absolute paths are not allowed"));
    }
    for c in p.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(deny("`..` is not allowed")),
            Component::RootDir | Component::Prefix(_) => {
                return Err(deny("absolute paths are not allowed"))
            }
        }
    }
    let target = workspace.join(p);

    // Walk up to the nearest existing ancestor and make sure it canonicalises
    // to somewhere inside the workspace.
    let ws_canon = workspace
        .canonicalize()
        .map_err(|e| AppError::internal(format!("workspace missing: {e}")))?;
    let mut probe = target.clone();
    let existing = loop {
        if probe.exists() {
            break probe;
        }
        match probe.parent() {
            Some(parent) => probe = parent.to_path_buf(),
            None => return Err(deny("path escapes the workspace")),
        }
    };
    let existing_canon = existing
        .canonicalize()
        .map_err(|e| AppError::internal(format!("path check failed: {e}")))?;
    if !existing_canon.starts_with(&ws_canon) {
        return Err(deny("path escapes the workspace"));
    }
    Ok(target)
}

// ───────────────────────── @-mentions (FR-T4) ─────────────────────────

/// Tab ids whose title is `@`-mentioned in `text`. Longest titles first so
/// `@Weekly review` wins over `@Weekly`.
pub fn parse_mentions(text: &str, tabs: &[(String, String)]) -> Vec<String> {
    let mut by_len: Vec<&(String, String)> = tabs.iter().collect();
    by_len.sort_by_key(|(_, title)| std::cmp::Reverse(title.chars().count()));
    let mut hit = Vec::new();
    for (id, title) in by_len {
        let title = title.trim();
        if !title.is_empty() && text.contains(&format!("@{title}")) {
            hit.push(id.clone());
        }
    }
    hit
}

// ───────────────────────── context budget (AC-6-9) ─────────────────────────

fn msg_len(m: &Value) -> usize {
    m.get("content").and_then(|c| c.as_str()).map(|s| s.len()).unwrap_or(0)
        + m.get("tool_calls").map(|t| t.to_string().len()).unwrap_or(0)
}

fn has_tool_calls(m: &Value) -> bool {
    m.get("tool_calls").map(|t| !t.is_null()).unwrap_or(false)
}

/// Trim `history` (OpenAI-shaped, no system message) to the budget. Everything
/// from the last `user` turn onward is kept verbatim; older turns are folded
/// into one `system` summary message. Returns `(messages_with_system, folded)`.
pub fn fit_budget(system: &str, history: Vec<Value>) -> (Vec<Value>, u32) {
    let sys = json!({ "role": "system", "content": system });
    let total: usize = system.len() + history.iter().map(msg_len).sum::<usize>();
    if total <= BUDGET_CHARS {
        let mut out = Vec::with_capacity(history.len() + 1);
        out.push(sys);
        out.extend(history);
        return (out, 0);
    }

    let tail_start = history
        .iter()
        .rposition(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .unwrap_or(0);
    let (head, tail) = history.split_at(tail_start);
    let mut head = head.to_vec();
    let tail = tail.to_vec();

    let tail_len: usize = tail.iter().map(msg_len).sum();
    let mut summary = String::new();
    let mut folded = 0u32;

    let fits = |head: &[Value], summary: &str| {
        system.len() + summary.len() + tail_len + head.iter().map(msg_len).sum::<usize>() <= BUDGET_CHARS
    };
    while !head.is_empty() && !fits(&head, &summary) {
        let m = head.remove(0);
        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("?");
        let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
        if summary.len() < 1600 && !content.is_empty() {
            summary.push_str(role);
            summary.push_str(": ");
            summary.push_str(&content.chars().take(400).collect::<String>());
            summary.push('\n');
        }
        folded += 1;
    }
    // Never start the kept head on an orphan tool reply, and never end it on an
    // assistant turn whose tool calls were folded away.
    while head.first().map(|m| m.get("role").and_then(|r| r.as_str()) == Some("tool")).unwrap_or(false) {
        head.remove(0);
        folded += 1;
    }
    while head.last().map(has_tool_calls).unwrap_or(false) {
        head.pop();
        folded += 1;
    }

    let mut out = Vec::with_capacity(head.len() + tail.len() + 2);
    out.push(sys);
    if folded > 0 && !summary.is_empty() {
        out.push(json!({
            "role": "system",
            "content": format!("Earlier conversation (condensed):\n{summary}"),
        }));
    }
    out.extend(head);
    out.extend(tail);
    (out, folded)
}

// ───────────────────────── tool dispatch ─────────────────────────

/// OpenAI `tools` array for the built-ins.
fn tool_defs() -> Value {
    let f = |name: &str, description: &str, props: Value, required: Value| {
        json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": { "type": "object", "properties": props, "required": required },
            }
        })
    };
    json!([
        f("search_sources", "Hybrid search over this project's sources.",
          json!({ "query": { "type": "string" }, "k": { "type": "integer" } }), json!(["query"])),
        f("read_document", "Read a source's extracted text. Omit page for the whole document.",
          json!({ "sourceId": { "type": "string" }, "page": { "type": "integer" } }), json!(["sourceId"])),
        f("list_sources", "List this project's sources.", json!({}), json!([])),
        f("list_tabs", "List the other Studio conversations in this project.", json!({}), json!([])),
        f("read_tab", "Read another Studio conversation by its title.",
          json!({ "title": { "type": "string" } }), json!(["title"])),
        f("list_files", "List files in this tab's workspace/.", json!({}), json!([])),
        f("read_file", "Read a file from this tab's workspace/.",
          json!({ "path": { "type": "string" } }), json!(["path"])),
        f("write_file", "Write a file into this tab's workspace/. Needs the reader's approval.",
          json!({ "path": { "type": "string" }, "content": { "type": "string" } }), json!(["path", "content"])),
    ])
}

/// Run one built-in tool. `arguments` is the raw JSON string from the model.
pub fn dispatch_tool(
    db: &Connection,
    workspace: &Path,
    thread_id: &str,
    name: &str,
    arguments: &str,
) -> AppResult<String> {
    let raw = if arguments.trim().is_empty() { "{}" } else { arguments };
    let args: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let n = |k: &str| args.get(k).and_then(|v| v.as_u64());

    match name {
        "search_sources" => {
            let query = s("query").unwrap_or_default();
            if query.trim().is_empty() {
                return Ok("(no query)".into());
            }
            let k = n("k").unwrap_or(8).clamp(1, 20) as usize;
            let hits = retrieval::hybrid_search(db, &query, None, None, k)?;
            if hits.is_empty() {
                return Ok("No matches.".into());
            }
            let mut out = String::new();
            for h in hits {
                out.push_str(&format!(
                    "- {} · p{} · source {}\n  {}\n",
                    h.source_name,
                    h.ordinal,
                    h.source_id,
                    h.snippet.replace('\n', " ")
                ));
            }
            Ok(out)
        }
        "read_document" => {
            let sid = s("sourceId").ok_or_else(|| tool_arg("sourceId"))?;
            let text: String = match n("page") {
                Some(p) => db
                    .query_row(
                        "SELECT text FROM documents WHERE source_id = ?1 AND ordinal = ?2",
                        params![sid, p as i64],
                        |r| r.get(0),
                    )
                    .optional()?
                    .unwrap_or_default(),
                None => {
                    let mut stmt = db.prepare(
                        "SELECT text FROM documents WHERE source_id = ?1 ORDER BY ordinal",
                    )?;
                    let parts: Vec<String> = stmt
                        .query_map([&sid], |r| r.get::<_, String>(0))?
                        .collect::<rusqlite::Result<_>>()?;
                    parts.join("\n\n")
                }
            };
            if text.is_empty() {
                return Ok("(no extracted text for that source/page)".into());
            }
            Ok(text.chars().take(8000).collect())
        }
        "list_sources" => {
            let mut stmt = db.prepare(
                "SELECT id, kind, original_name, status FROM sources ORDER BY added_at",
            )?;
            let rows: Vec<String> = stmt
                .query_map([], |r| {
                    Ok(format!(
                        "- {} · {} · {} ({})",
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?
                    ))
                })?
                .collect::<rusqlite::Result<_>>()?;
            Ok(if rows.is_empty() { "(no sources)".into() } else { rows.join("\n") })
        }
        "list_tabs" => {
            let mut stmt = db.prepare("SELECT title FROM studio_tabs ORDER BY ordinal")?;
            let rows: Vec<String> = stmt
                .query_map([], |r| Ok(format!("- {}", r.get::<_, String>(0)?)))?
                .collect::<rusqlite::Result<_>>()?;
            Ok(if rows.is_empty() { "(no tabs)".into() } else { rows.join("\n") })
        }
        "read_tab" => {
            let title = s("title").ok_or_else(|| tool_arg("title"))?;
            let tid: Option<String> = db
                .query_row(
                    "SELECT thread_id FROM studio_tabs WHERE title = ?1 COLLATE NOCASE",
                    [&title],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(tid) = tid else { return Ok(format!("(no tab titled \"{title}\")")) };
            let msgs = load_messages(db, &tid)?;
            let dump: String = msgs
                .iter()
                .filter(|m| m.role != "tool")
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n\n");
            Ok(dump.chars().take(6000).collect())
        }
        "list_files" => {
            let mut found = Vec::new();
            walk_workspace(workspace, workspace, &mut found);
            Ok(if found.is_empty() { "(workspace is empty)".into() } else { found.join("\n") })
        }
        "read_file" => {
            let path = s("path").ok_or_else(|| tool_arg("path"))?;
            let abs = resolve_in_workspace(workspace, &path)?;
            let bytes = std::fs::read(&abs)
                .map_err(|_| AppError::new("STUDIO_FILE_NOT_FOUND", "error.studio.fileNotFound", &path))?;
            let text = String::from_utf8_lossy(&bytes);
            Ok(text.chars().take(20_000).collect())
        }
        "write_file" => {
            let path = s("path").ok_or_else(|| tool_arg("path"))?;
            let content = s("content").unwrap_or_default();
            let abs = resolve_in_workspace(workspace, &path)?;
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&abs, content.as_bytes())?;
            let bytes = content.len() as i64;
            let rel_path = format!("workspace/{}", path.trim_start_matches("./"));
            let mime = mime_guess_ext(&abs);
            db.execute(
                "INSERT INTO artifacts (id, thread_id, rel_path, bytes, mime, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(rel_path) DO UPDATE SET
                   bytes = excluded.bytes, mime = excluded.mime,
                   thread_id = excluded.thread_id, created_at = excluded.created_at",
                params![Uuid::now_v7().to_string(), thread_id, rel_path, bytes, mime, now_iso8601()],
            )?;
            Ok(format!("wrote {rel_path} ({bytes} bytes)"))
        }
        other => Err(AppError::new(
            "STUDIO_UNKNOWN_TOOL",
            "error.studio.unknownTool",
            format!("no such tool: {other}"),
        )),
    }
}

fn tool_arg(name: &str) -> AppError {
    AppError::new("STUDIO_TOOL_ARG", "error.studio.toolArg", format!("missing argument: {name}"))
}

fn walk_workspace(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_workspace(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            out.push(format!("- {} ({size} B)", rel.to_string_lossy()));
        }
    }
}

fn mime_guess_ext(path: &Path) -> Option<String> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("md") => Some("text/markdown".into()),
        Some("txt") => Some("text/plain".into()),
        Some("json") => Some("application/json".into()),
        Some("csv") => Some("text/csv".into()),
        Some("html") => Some("text/html".into()),
        _ => None,
    }
}

// ───────────────────────── send / resume loop ─────────────────────────

struct LoopCtx {
    projects_root: PathBuf,
    project_id: String,
    thread_id: String,
    workspace: PathBuf,
    model: String,
    base_params: Value,
    system: String,
    ctx_items: Vec<retrieval::HybridHit>,
}

pub async fn send(
    reg: &crate::services::ai::StreamRegistry,
    app_db_path: &Path,
    projects_root: &Path,
    input: StudioSendInput,
    ui_lang: String,
) -> AppResult<StudioSendResult> {
    let StudioSendInput { project_id, tab_id, text, scope } = input;
    let text = text.trim().to_string();

    // Resolve everything synchronously, then drop all DB connections before the
    // first await (a rusqlite Connection is not Send).
    let (resolved, embed_role, ctx, resume_only) = {
        let app_db = crate::storage::open(app_db_path)?;
        let resolved = profiles::resolve(&app_db, Role::Chat)?.ok_or_else(|| {
            AppError::new("AI_NOT_CONFIGURED", "error.ai.notConfigured", "no chat model")
        })?;
        let embed_role = profiles::resolve(&app_db, Role::Embedding)?;
        let project_name: String = app_db
            .query_row("SELECT name FROM projects WHERE id = ?1", [&project_id], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        drop(app_db);

        let db = projects::open_db(projects_root, &project_id)?;
        let thread_id = tab_thread(&db, &tab_id)?;

        // "続行": empty text + a trailing round-cap message -> resume, no new turn.
        let last_status: Option<String> = db
            .query_row(
                "SELECT status FROM messages WHERE thread_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
                [&thread_id],
                |r| r.get(0),
            )
            .optional()?;
        let resume_only = text.is_empty() && last_status.as_deref() == Some("needs_continue");
        if text.is_empty() && !resume_only {
            return Err(AppError::new(
                "STUDIO_EMPTY_MESSAGE",
                "error.studio.emptyMessage",
                "message is required",
            ));
        }

        db.execute("UPDATE studio_tabs SET scope = ?2 WHERE id = ?1", params![tab_id, scope])?;

        let source_filter = scope.strip_prefix("source:").map(str::to_string);

        // @-mentioned tabs -> transcripts appended to the system context.
        let tabs: Vec<(String, String)> = {
            let mut stmt = db.prepare("SELECT thread_id, title FROM studio_tabs WHERE thread_id != ?1")?;
            let v: Vec<(String, String)> = stmt
                .query_map([&thread_id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            v
        };
        let mention_threads = parse_mentions(&text, &tabs);
        let mut mention_block = String::new();
        for mt in &mention_threads {
            let title: String = db
                .query_row("SELECT title FROM studio_tabs WHERE thread_id = ?1", [mt], |r| r.get(0))
                .unwrap_or_default();
            let dump: String = load_messages(&db, mt)?
                .iter()
                .filter(|m| m.role != "tool")
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");
            mention_block.push_str(&format!("\n\n### @{title}\n{}", dump.chars().take(3000).collect::<String>()));
        }

        // RAG over the tab's scope, from the new question (or the last one on resume).
        let query_text = if resume_only { last_user_text(&db, &thread_id)? } else { text.clone() };
        let ctx_items = if query_text.is_empty() {
            Vec::new()
        } else {
            let qvec = match embed_role.clone() {
                Some(role) => crate::services::ai::embed_with(role, std::slice::from_ref(&query_text), true)
                    .await
                    .ok()
                    .and_then(|(_, mut v)| v.pop()),
                None => None,
            };
            let db2 = projects::open_db(projects_root, &project_id)?;
            let hits = retrieval::hybrid_search(
                &db2,
                &query_text,
                qvec.as_deref(),
                source_filter.as_deref(),
                RAG_TOP_K,
            )?;
            drop(db2);
            hits
        };

        // Persist the user's turn (unless this is a bare "続行").
        if !resume_only {
            db.execute(
                "INSERT INTO messages (id, thread_id, role, content, created_at)
                 VALUES (?1, ?2, 'user', ?3, ?4)",
                params![Uuid::now_v7().to_string(), thread_id, text, now_iso8601()],
            )?;
            db.execute("UPDATE threads SET updated_at = ?2 WHERE id = ?1", params![thread_id, now_iso8601()])?;
        }

        let system = format!(
            "{}\n\n[Workspace] Files you write go to `{}/workspace/`. Use relative paths.{}\n\n{}",
            prompts::studio(&ui_lang).replace("{{project}}", &project_name),
            project_name,
            mention_block,
            build_rag_block(&ctx_items, &ui_lang),
        );

        let workspace = projects::project_dir(projects_root, &project_id).join("workspace");
        std::fs::create_dir_all(&workspace).ok();

        (
            resolved.clone(),
            embed_role,
            LoopCtx {
                projects_root: projects_root.to_path_buf(),
                project_id: project_id.clone(),
                thread_id,
                workspace,
                model: resolved.model.clone(),
                base_params: resolved.params.clone(),
                system,
                ctx_items,
            },
            resume_only,
        )
    };
    let _ = (embed_role, resume_only);

    let client = build_client(&resolved)?;
    let token = reg.start_keyed(&format!("studio:{tab_id}"));
    let result = run_loop(&ctx, &client, &token, 0).await;
    reg.finish(&format!("studio:{tab_id}"));
    result
}

pub async fn resolve_tool(
    reg: &crate::services::ai::StreamRegistry,
    app_db_path: &Path,
    projects_root: &Path,
    project_id: String,
    tab_id: String,
    approved: bool,
    ui_lang: String,
) -> AppResult<StudioSendResult> {
    let (resolved, ctx) = {
        let app_db = crate::storage::open(app_db_path)?;
        let resolved = profiles::resolve(&app_db, Role::Chat)?.ok_or_else(|| {
            AppError::new("AI_NOT_CONFIGURED", "error.ai.notConfigured", "no chat model")
        })?;
        let embed_role = profiles::resolve(&app_db, Role::Embedding)?;
        let project_name: String = app_db
            .query_row("SELECT name FROM projects WHERE id = ?1", [&project_id], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        drop(app_db);

        let db = projects::open_db(projects_root, &project_id)?;
        let thread_id = tab_thread(&db, &tab_id)?;
        let scope: String = db
            .query_row("SELECT scope FROM studio_tabs WHERE id = ?1", [&tab_id], |r| r.get(0))
            .optional()?
            .unwrap_or_else(|| "project".into());

        // The paused assistant turn: last message with tool_calls awaiting us.
        let pending: Option<(String, String, String)> = db
            .query_row(
                "SELECT id, content, tool_calls FROM messages
                 WHERE thread_id = ?1 AND tool_calls IS NOT NULL
                   AND status IN ('pending_approval', 'needs_continue')
                 ORDER BY created_at DESC, id DESC LIMIT 1",
                [&thread_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((msg_id, _content, calls_json)) = pending else {
            return Err(AppError::new(
                "STUDIO_NOTHING_PENDING",
                "error.studio.nothingPending",
                "no tool call is waiting",
            ));
        };
        let calls: Vec<Value> = serde_json::from_str(&calls_json).unwrap_or_default();

        // Execute (or record the denial), then clear the pause.
        for c in &calls {
            let id = c.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let cname = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let cargs = c.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
            let workspace = projects::project_dir(projects_root, &project_id).join("workspace");
            let out = if approved {
                match dispatch_tool(&db, &workspace, &thread_id, cname, cargs) {
                    Ok(s) => s,
                    Err(e) => format!("ERROR: {}", e.message),
                }
            } else {
                "The reader denied this tool call.".to_string()
            };
            db.execute(
                "INSERT INTO messages (id, thread_id, role, content, tool_call_id, status, created_at)
                 VALUES (?1, ?2, 'tool', ?3, ?4, 'complete', ?5)",
                params![Uuid::now_v7().to_string(), thread_id, out, id, now_iso8601()],
            )?;
        }
        db.execute("UPDATE messages SET status = 'complete' WHERE id = ?1", [&msg_id])?;

        // Rebuild RAG context from the last question so citations still resolve.
        let query_text = last_user_text(&db, &thread_id)?;
        let source_filter = scope.strip_prefix("source:").map(str::to_string);
        let ctx_items = if query_text.is_empty() {
            Vec::new()
        } else {
            let qvec = match embed_role {
                Some(role) => crate::services::ai::embed_with(role, std::slice::from_ref(&query_text), true)
                    .await
                    .ok()
                    .and_then(|(_, mut v)| v.pop()),
                None => None,
            };
            let db2 = projects::open_db(projects_root, &project_id)?;
            let hits = retrieval::hybrid_search(&db2, &query_text, qvec.as_deref(), source_filter.as_deref(), RAG_TOP_K)?;
            drop(db2);
            hits
        };

        let system = format!(
            "{}\n\n[Workspace] Files you write go to `{}/workspace/`. Use relative paths.\n\n{}",
            prompts::studio(&ui_lang).replace("{{project}}", &project_name),
            project_name,
            build_rag_block(&ctx_items, &ui_lang),
        );
        let workspace = projects::project_dir(projects_root, &project_id).join("workspace");

        (
            resolved.clone(),
            LoopCtx {
                projects_root: projects_root.to_path_buf(),
                project_id: project_id.clone(),
                thread_id,
                workspace,
                model: resolved.model.clone(),
                base_params: resolved.params.clone(),
                system,
                ctx_items,
            },
        )
    };

    let client = build_client(&resolved)?;
    let token = reg.start_keyed(&format!("studio:{tab_id}"));
    let result = run_loop(&ctx, &client, &token, 0).await;
    reg.finish(&format!("studio:{tab_id}"));
    result
}

fn build_client(r: &ResolvedRole) -> AppResult<AiClient> {
    AiClient::new(&r.base_url, r.api_key.clone(), r.extra_headers.clone(), r.timeout_ms)
}

fn last_user_text(db: &Connection, thread_id: &str) -> AppResult<String> {
    Ok(db
        .query_row(
            "SELECT content FROM messages WHERE thread_id = ?1 AND role = 'user'
             ORDER BY created_at DESC, id DESC LIMIT 1",
            [thread_id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_default())
}

/// The agentic loop. Reads/writes `messages` each pass so it can be resumed
/// across the IPC boundary; never holds a connection across the model await.
async fn run_loop(
    ctx: &LoopCtx,
    client: &AiClient,
    token: &CancellationToken,
    start_round: u32,
) -> AppResult<StudioSendResult> {
    let mut round = start_round;
    let mut summarised_total = 0u32;

    loop {
        if token.is_cancelled() {
            return Ok(result(round, false, false, true, summarised_total));
        }

        // Build the request from persisted history.
        let (messages, folded) = {
            let db = projects::open_db(&ctx.projects_root, &ctx.project_id)?;
            let history = openai_history(&db, &ctx.thread_id)?;
            drop(db);
            fit_budget(&ctx.system, history)
        };
        summarised_total += folded;

        let mut params = ctx.base_params.clone();
        params["tools"] = tool_defs();
        params["tool_choice"] = json!("auto");

        let acc = std::sync::Mutex::new(String::new());
        let (usage, _truncated, calls) = client
            .chat_stream(&ctx.model, json!(messages), &params, token, |kind, t| {
                if kind == "text" {
                    acc.lock().unwrap().push_str(t);
                }
            })
            .await?;
        let text = acc.into_inner().unwrap();

        if token.is_cancelled() {
            persist_assistant(ctx, &text, &[], "cancelled", usage.as_ref())?;
            return Ok(result(round, false, false, true, summarised_total));
        }

        // Plain answer -> resolve citations, persist, done.
        if calls.is_empty() {
            let citations = resolve_citations(&text, &ctx.ctx_items);
            persist_assistant_final(ctx, &text, &citations, "complete", usage.as_ref())?;
            return Ok(result(round + 1, false, false, false, summarised_total));
        }

        let calls_json: Vec<Value> = calls
            .iter()
            .map(|c| json!({ "id": c.id, "name": c.name, "arguments": c.arguments }))
            .collect();
        let needs_approval = calls.iter().any(|c| requires_approval(&c.name));

        // 10-round cap (AC-6-8): stop, leave the proposal for "続行".
        if round >= MAX_TOOL_ROUNDS && !needs_approval {
            persist_assistant(ctx, &text, &calls_json, "needs_continue", usage.as_ref())?;
            return Ok(result(round, true, false, false, summarised_total));
        }
        if needs_approval {
            persist_assistant(ctx, &text, &calls_json, "pending_approval", usage.as_ref())?;
            return Ok(result(round, false, true, false, summarised_total));
        }

        // Auto-approved: persist the proposal, run each tool, append results.
        persist_assistant(ctx, &text, &calls_json, "complete", usage.as_ref())?;
        {
            let db = projects::open_db(&ctx.projects_root, &ctx.project_id)?;
            for c in &calls {
                let out = match dispatch_tool(&db, &ctx.workspace, &ctx.thread_id, &c.name, &c.arguments) {
                    Ok(s) => s,
                    Err(e) => format!("ERROR: {}", e.message),
                };
                db.execute(
                    "INSERT INTO messages (id, thread_id, role, content, tool_call_id, status, created_at)
                     VALUES (?1, ?2, 'tool', ?3, ?4, 'complete', ?5)",
                    params![Uuid::now_v7().to_string(), ctx.thread_id, out, c.id, now_iso8601()],
                )?;
            }
        }
        round += 1;
    }
}

fn result(iterations: u32, needs_continue: bool, awaiting_approval: bool, cancelled: bool, summarised: u32) -> StudioSendResult {
    StudioSendResult {
        iterations,
        needs_continue,
        awaiting_approval,
        cancelled,
        summarised_messages: summarised,
    }
}

/// Persisted `messages` -> OpenAI-shaped array (no system message).
fn openai_history(db: &Connection, thread_id: &str) -> AppResult<Vec<Value>> {
    let mut stmt = db.prepare(
        "SELECT role, content, tool_calls, tool_call_id FROM messages
         WHERE thread_id = ?1 ORDER BY created_at, id",
    )?;
    let rows: Vec<(String, String, Option<String>, Option<String>)> = stmt
        .query_map([thread_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    let mut out = Vec::with_capacity(rows.len());
    for (role, content, tool_calls, tool_call_id) in rows {
        match role.as_str() {
            "tool" => out.push(json!({
                "role": "tool",
                "content": content,
                "tool_call_id": tool_call_id.unwrap_or_default(),
            })),
            "assistant" => {
                let mut m = json!({ "role": "assistant", "content": content });
                if let Some(tc) = tool_calls.as_deref().and_then(|s| serde_json::from_str::<Vec<Value>>(s).ok()) {
                    let mapped: Vec<Value> = tc
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.get("id").and_then(|v| v.as_str()).unwrap_or_default(),
                                "type": "function",
                                "function": {
                                    "name": c.get("name").and_then(|v| v.as_str()).unwrap_or_default(),
                                    "arguments": c.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}"),
                                }
                            })
                        })
                        .collect();
                    if !mapped.is_empty() {
                        m["tool_calls"] = json!(mapped);
                    }
                }
                out.push(m);
            }
            _ => out.push(json!({ "role": "user", "content": content })),
        }
    }
    Ok(out)
}

fn persist_assistant(
    ctx: &LoopCtx,
    text: &str,
    tool_calls: &[Value],
    status: &str,
    usage: Option<&crate::domain::ai::TokenUsage>,
) -> AppResult<()> {
    let db = projects::open_db(&ctx.projects_root, &ctx.project_id)?;
    db.execute(
        "INSERT INTO messages (id, thread_id, role, content, tool_calls, model, usage, status, created_at)
         VALUES (?1, ?2, 'assistant', ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            Uuid::now_v7().to_string(),
            ctx.thread_id,
            text,
            if tool_calls.is_empty() { None } else { Some(json!(tool_calls).to_string()) },
            ctx.model,
            usage.map(|u| json!({ "promptTokens": u.prompt_tokens, "completionTokens": u.completion_tokens }).to_string()),
            status,
            now_iso8601(),
        ],
    )?;
    db.execute("UPDATE threads SET updated_at = ?2 WHERE id = ?1", params![ctx.thread_id, now_iso8601()])?;
    Ok(())
}

fn persist_assistant_final(
    ctx: &LoopCtx,
    text: &str,
    citations: &[Citation],
    status: &str,
    usage: Option<&crate::domain::ai::TokenUsage>,
) -> AppResult<()> {
    let db = projects::open_db(&ctx.projects_root, &ctx.project_id)?;
    db.execute(
        "INSERT INTO messages (id, thread_id, role, content, citations, model, usage, status, created_at)
         VALUES (?1, ?2, 'assistant', ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            Uuid::now_v7().to_string(),
            ctx.thread_id,
            text,
            serde_json::to_string(citations).unwrap_or_else(|_| "[]".into()),
            ctx.model,
            usage.map(|u| json!({ "promptTokens": u.prompt_tokens, "completionTokens": u.completion_tokens }).to_string()),
            status,
            now_iso8601(),
        ],
    )?;
    db.execute("UPDATE threads SET updated_at = ?2 WHERE id = ?1", params![ctx.thread_id, now_iso8601()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_guard_rejects_escapes() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        for bad in ["/etc/passwd", "../secret", "a/../../b", "~/x", "", "  "] {
            assert!(resolve_in_workspace(ws, bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn workspace_guard_allows_simple_relative() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        let p = resolve_in_workspace(ws, "notes/day1.md").unwrap();
        assert!(p.starts_with(ws));
        assert!(p.ends_with("notes/day1.md"));
    }

    #[test]
    fn mentions_match_longest_title_first() {
        let tabs = vec![
            ("a".to_string(), "Weekly".to_string()),
            ("b".to_string(), "Weekly review".to_string()),
        ];
        let hit = parse_mentions("see @Weekly review please", &tabs);
        assert_eq!(hit, vec!["b", "a"]);
    }

    #[test]
    fn fit_budget_keeps_latest_user_turn() {
        let filler = "x".repeat(20_000);
        let mut history = Vec::new();
        for _ in 0..6 {
            history.push(json!({ "role": "user", "content": filler }));
            history.push(json!({ "role": "assistant", "content": filler }));
        }
        history.push(json!({ "role": "user", "content": "THE LATEST QUESTION" }));
        let (msgs, folded) = fit_budget("sys", history);
        assert!(folded > 0, "older turns should be folded");
        let last = msgs.last().unwrap();
        assert_eq!(last["content"], "THE LATEST QUESTION");
        let total: usize = msgs
            .iter()
            .map(|m| m["content"].as_str().map(|s| s.len()).unwrap_or(0))
            .sum();
        assert!(total <= BUDGET_CHARS + 2_000, "trimmed under budget, got {total}");
    }

    #[test]
    fn fit_budget_noop_when_small() {
        let history = vec![
            json!({ "role": "user", "content": "hi" }),
            json!({ "role": "assistant", "content": "hello" }),
        ];
        let (msgs, folded) = fit_budget("sys", history);
        assert_eq!(folded, 0);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0]["role"], "system");
    }
}
