//! Typed interactive visuals shared by Studio and Live (plan §4).
//! Minimal contract: {id, schemaVersion, title, html, css, js, data,
//! aspectRatio, sourceRefs, initialState}. HTML/CSS/JS total ≤ 256 KiB,
//! state ≤ 16 KiB. No remote URLs, CDN, external fonts, import(), network
//! fetch or form submission in v1.6.0. Served to an opaque-origin
//! `sandbox="allow-scripts"` iframe; the parent IPC bridge and project asset
//! URLs are never passed in.

use crate::domain::visual::{VisualCreate, VisualPreview};
use crate::error::{AppError, AppResult};
use crate::services::projects;
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

pub const VISUAL_CODE_MAX: usize = 256 * 1024;
pub const VISUAL_STATE_MAX: usize = 16 * 1024;
pub const VISUAL_SCHEMA_VERSION: i32 = 1;

fn row_to_visual(r: &rusqlite::Row) -> rusqlite::Result<VisualPreview> {
    let data_s: String = r.get("data_json")?;
    let refs_s: String = r.get("source_refs")?;
    let state_s: String = r.get("initial_state")?;
    Ok(VisualPreview {
        id: r.get("id")?,
        schema_version: r.get("schema_version")?,
        title: r.get("title")?,
        html: r.get("html")?,
        css: r.get("css")?,
        js: r.get("js")?,
        data_json: serde_json::from_str(&data_s).unwrap_or(serde_json::Value::Null),
        aspect_ratio: r.get("aspect_ratio")?,
        source_refs: serde_json::from_str(&refs_s).unwrap_or(serde_json::Value::Null),
        initial_state: serde_json::from_str(&state_s).unwrap_or(serde_json::Value::Null),
        model: r.get("model")?,
        created_at: r.get("created_at")?,
    })
}

/// Shared type/size/boundary validation used by the Studio tool, the Live
/// command and the unit tests. Rejects what v1.6.0 never executes.
pub fn validate_parts(
    title: &str,
    html: &str,
    css: &str,
    js: &str,
    data: &serde_json::Value,
    state: &serde_json::Value,
) -> AppResult<()> {
    if title.chars().count() > 200 {
        return Err(AppError::new(
            "VISUAL_TOO_LARGE",
            "error.visual.tooLarge",
            "title exceeds 200 chars",
        ));
    }
    let total = html.len() + css.len() + js.len();
    if total > VISUAL_CODE_MAX {
        return Err(AppError::new(
            "VISUAL_TOO_LARGE",
            "error.visual.tooLarge",
            format!("html+css+js exceeds {} bytes", VISUAL_CODE_MAX),
        ));
    }
    if state.to_string().len() > VISUAL_STATE_MAX {
        return Err(AppError::new(
            "VISUAL_TOO_LARGE",
            "error.visual.tooLarge",
            format!("initial state exceeds {} bytes", VISUAL_STATE_MAX),
        ));
    }
    if data.to_string().len() > VISUAL_STATE_MAX {
        return Err(AppError::new(
            "VISUAL_TOO_LARGE",
            "error.visual.tooLarge",
            "data exceeds 16 KiB",
        ));
    }
    let code = format!("{html}\n{css}\n{js}");
    let lower = code.to_lowercase();
    // Blocklist is deliberately narrow and documented: v1.6.0 visuals are
    // offline 2D diagrams (SVG/Canvas + inline script). Anything needing the
    // network, modules, or DOM escape is rejected with a reason.
    for needle in [
        "http://",
        "https://",
        "//cdn.",
        "cdn.jsdelivr",
        "unpkg.com",
        "import(",
        "import ",
        "fetch(",
        "xmlhttprequest",
        "websocket",
        "eventsource(",
        "<form",
        "window.top",
        "window.parent",
        "parent.postmessage",
        "__tauri",
        "tauri-internals",
        "__tauri_internals__",
        "invoke(",
    ] {
        if lower.contains(needle) {
            return Err(AppError::new(
                "VISUAL_FORBIDDEN",
                "error.visual.forbidden",
                format!("visual code must not contain `{needle}` in v1.6.0"),
            ));
        }
    }
    // No form submission / navigation targets.
    if lower.contains("<iframe") || lower.contains("frame-src") {
        return Err(AppError::new(
            "VISUAL_FORBIDDEN",
            "error.visual.forbidden",
            "nested frames are not allowed in v1.6.0 visuals",
        ));
    }
    Ok(())
}

pub fn create(
    projects_root: &Path,
    project_id: &str,
    input: &VisualCreate,
) -> AppResult<VisualPreview> {
    let db = projects::open_db(projects_root, project_id)?;
    let data = input.data_json.clone().unwrap_or(serde_json::json!({}));
    let state = input.initial_state.clone().unwrap_or(serde_json::json!({}));
    let refs = input.source_refs.clone().unwrap_or(serde_json::json!([]));
    let created = create_on_db(
        &db,
        &input.title,
        &input.html,
        &input.css,
        &input.js,
        &data,
        &input.aspect_ratio,
        &refs,
        &state,
        &input.model,
    )?;
    if let Some(mid) = &input.message_id {
        // Link to the conversation turn; old messages stay readable without it.
        let ordinal: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM message_visuals WHERE message_id = ?1",
                [mid],
                |r| r.get(0),
            )
            .unwrap_or(0);
        // Best-effort: a missing message row (foreign key) must not turn a
        // stored visual into a failure.
        let _ = db.execute(
            "INSERT OR IGNORE INTO message_visuals (message_id, visual_id, ordinal) VALUES (?1, ?2, ?3)",
            params![mid, created.id, ordinal],
        );
    }
    Ok(created)
}

/// Direct insert on an already-open project connection (Studio tool loop).
#[allow(clippy::too_many_arguments)]
pub fn create_on_db(
    db: &rusqlite::Connection,
    title: &str,
    html: &str,
    css: &str,
    js: &str,
    data: &serde_json::Value,
    aspect_ratio: &str,
    refs: &serde_json::Value,
    state: &serde_json::Value,
    model: &str,
) -> AppResult<VisualPreview> {
    validate_parts(title, html, css, js, data, state)?;
    if !refs.is_array() {
        return Err(AppError::new(
            "VISUAL_INVALID",
            "error.visual.invalid",
            "sourceRefs must be an array",
        ));
    }
    // Resolve citations through the existing Rust-side resolver shape: every
    // ref must be {sourceId, ordinal?} pointing at a real source; the figure
    // text itself is never trusted as a citation.
    if let Some(arr) = refs.as_array() {
        for r in arr {
            let sid = r.get("sourceId").and_then(|v| v.as_str()).unwrap_or("");
            if sid.is_empty() {
                return Err(AppError::new(
                    "VISUAL_INVALID",
                    "error.visual.invalid",
                    "sourceRefs entries need a sourceId",
                ));
            }
            let exists: Option<String> = db
                .query_row("SELECT id FROM sources WHERE id = ?1", [sid], |row| {
                    row.get(0)
                })
                .optional()?;
            if exists.is_none() {
                return Err(AppError::new(
                    "VISUAL_INVALID_REF",
                    "error.visual.invalidRef",
                    format!("unknown source in sourceRefs: {sid}"),
                ));
            }
        }
    }
    let aspect = if aspect_ratio.trim().is_empty() {
        "16:9".to_string()
    } else {
        aspect_ratio.to_string()
    };
    let id = Uuid::now_v7().to_string();
    let now = now_iso8601();
    db.execute(
        "INSERT INTO visual_previews (id, schema_version, title, html, css, js, data_json, aspect_ratio, source_refs, initial_state, model, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            VISUAL_SCHEMA_VERSION,
            title,
            html,
            css,
            js,
            data.to_string(),
            aspect,
            refs.to_string(),
            state.to_string(),
            model,
            now,
        ],
    )?;
    db.query_row(
        "SELECT * FROM visual_previews WHERE id = ?1",
        [&id],
        row_to_visual,
    )
    .optional()?
    .ok_or_else(|| AppError::new("VISUAL_NOT_FOUND", "error.visual.notFound", id.clone()))
}

pub fn get(projects_root: &Path, project_id: &str, visual_id: &str) -> AppResult<VisualPreview> {
    let db = projects::open_db(projects_root, project_id)?;
    db.query_row(
        "SELECT * FROM visual_previews WHERE id = ?1",
        [visual_id],
        row_to_visual,
    )
    .optional()?
    .ok_or_else(|| AppError::new("VISUAL_NOT_FOUND", "error.visual.notFound", visual_id))
}

pub fn list_for_message(
    projects_root: &Path,
    project_id: &str,
    message_id: &str,
) -> AppResult<Vec<VisualPreview>> {
    let db = projects::open_db(projects_root, project_id)?;
    let mut stmt = db.prepare(
        "SELECT v.* FROM visual_previews v JOIN message_visuals m ON m.visual_id = v.id
         WHERE m.message_id = ?1 ORDER BY m.ordinal ASC",
    )?;
    let rows = stmt
        .query_map([message_id], row_to_visual)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_network_module_and_escape_vectors() {
        let ok = serde_json::json!({});
        assert!(validate_parts("t", "<svg></svg>", "", "console.log(1)", &ok, &ok).is_ok());
        assert!(validate_parts(
            "t",
            "<script src=\"https://cdn.example/x.js\"></script>",
            "",
            "",
            &ok,
            &ok
        )
        .is_err());
        assert!(validate_parts("t", "", "", "fetch('/x')", &ok, &ok).is_err());
        assert!(validate_parts("t", "", "", "import('x')", &ok, &ok).is_err());
        assert!(validate_parts("t", "", "", "window.top.location", &ok, &ok).is_err());
        assert!(
            validate_parts("t", "", "", "window.__TAURI_INTERNALS__.invoke", &ok, &ok).is_err()
        );
        let big_state = serde_json::json!({ "blob": "x".repeat(VISUAL_STATE_MAX + 1) });
        assert!(validate_parts("t", "<p>a</p>", "", "", &ok, &big_state).is_err());
    }
}
