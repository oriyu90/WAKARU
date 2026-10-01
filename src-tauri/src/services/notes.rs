//! Sticky notes bound to a source + locator (plan §5).
//! Viewer-private memos: page/slide/image anchors use normalized x/y in
//! [0,1] on the source surface; reflowable text uses section/offset +
//! fingerprint with a "needs position check" fallback. Never fed to AI.

use crate::domain::notes::{Note, NoteCreate, NoteUpdate};
use crate::error::{AppError, AppResult};
use crate::services::projects;
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

pub const NOTE_BODY_MAX: usize = 4000;
const NOTE_COLORS: [&str; 4] = ["yellow", "pink", "blue", "green"];
const ANCHOR_KINDS: [&str; 3] = ["page", "text", "section"];

fn row_to_note(r: &rusqlite::Row) -> rusqlite::Result<Note> {
    let locator_s: String = r.get("locator")?;
    let anchor_s: String = r.get("anchor_json")?;
    Ok(Note {
        id: r.get("id")?,
        source_id: r.get("source_id")?,
        locator: serde_json::from_str(&locator_s).unwrap_or(serde_json::Value::Null),
        anchor_kind: r.get("anchor_kind")?,
        anchor_json: serde_json::from_str(&anchor_s).unwrap_or(serde_json::Value::Null),
        body: r.get("body")?,
        color: r.get("color")?,
        stack_order: r.get("stack_order")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
        deleted_at: r.get("deleted_at")?,
    })
}

fn ensure_source(db: &Connection, source_id: &str) -> AppResult<()> {
    let exists: Option<String> = db
        .query_row("SELECT id FROM sources WHERE id = ?1", [source_id], |r| {
            r.get(0)
        })
        .optional()?;
    if exists.is_none() {
        return Err(AppError::new(
            "SOURCE_NOT_FOUND",
            "error.source.notFound",
            source_id,
        ));
    }
    Ok(())
}

fn validate_anchor(kind: &str, anchor: &serde_json::Value) -> AppResult<()> {
    if !ANCHOR_KINDS.contains(&kind) {
        return Err(AppError::new(
            "NOTE_INVALID_ANCHOR",
            "error.note.invalidAnchor",
            format!("unknown anchor kind: {kind}"),
        ));
    }
    if !anchor.is_object() && !anchor.is_null() {
        return Err(AppError::new(
            "NOTE_INVALID_ANCHOR",
            "error.note.invalidAnchor",
            "anchor must be a JSON object",
        ));
    }
    if kind == "page" && anchor.is_object() {
        let obj = anchor.as_object().unwrap();
        for key in ["x", "y"] {
            if let Some(v) = obj.get(key) {
                let n = v.as_f64().ok_or_else(|| {
                    AppError::new(
                        "NOTE_INVALID_ANCHOR",
                        "error.note.invalidAnchor",
                        format!("{key} must be a number in [0,1]"),
                    )
                })?;
                if !n.is_finite() || !(0.0..=1.0).contains(&n) {
                    return Err(AppError::new(
                        "NOTE_INVALID_ANCHOR",
                        "error.note.invalidAnchor",
                        format!("{key} must be in [0,1]"),
                    ));
                }
            }
        }
        if let Some(p) = obj.get("page") {
            if !(p.is_u64() || p.is_i64()) {
                return Err(AppError::new(
                    "NOTE_INVALID_ANCHOR",
                    "error.note.invalidAnchor",
                    "page must be an integer",
                ));
            }
        }
    }
    Ok(())
}

fn validate_body(body: &str) -> AppResult<()> {
    if body.chars().count() > NOTE_BODY_MAX {
        return Err(AppError::new(
            "NOTE_TOO_LONG",
            "error.note.tooLong",
            format!("note body exceeds {NOTE_BODY_MAX} chars"),
        ));
    }
    Ok(())
}

fn validate_color(color: &str) -> AppResult<()> {
    if !NOTE_COLORS.contains(&color) {
        return Err(AppError::new(
            "NOTE_INVALID_COLOR",
            "error.note.invalidColor",
            format!("unknown note color: {color}"),
        ));
    }
    Ok(())
}

fn next_stack_order(db: &Connection, source_id: &str, locator: &str) -> i64 {
    db.query_row(
        "SELECT COALESCE(MAX(stack_order), -1) + 1 FROM notes WHERE source_id = ?1 AND locator = ?2 AND deleted_at IS NULL",
        params![source_id, locator],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn list(projects_root: &Path, project_id: &str, source_id: &str) -> AppResult<Vec<Note>> {
    let db = projects::open_db(projects_root, project_id)?;
    ensure_source(&db, source_id)?;
    let mut stmt = db.prepare(
        "SELECT * FROM notes WHERE source_id = ?1 AND deleted_at IS NULL ORDER BY stack_order ASC, created_at ASC",
    )?;
    let rows = stmt
        .query_map([source_id], row_to_note)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(projects_root: &Path, project_id: &str, input: &NoteCreate) -> AppResult<Note> {
    let db = projects::open_db(projects_root, project_id)?;
    ensure_source(&db, &input.source_id)?;
    validate_body(&input.body)?;
    validate_color(&input.color)?;
    let anchor = input.anchor_json.clone().unwrap_or(serde_json::json!({}));
    validate_anchor(&input.anchor_kind, &anchor)?;
    let locator_v = input.locator.clone().unwrap_or(serde_json::json!({}));
    let locator_s = locator_v.to_string();
    let now = now_iso8601();
    let id = Uuid::now_v7().to_string();
    let stack = next_stack_order(&db, &input.source_id, &locator_s);
    db.execute(
        "INSERT INTO notes (id, source_id, locator, anchor_kind, anchor_json, body, color, stack_order, created_at, updated_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)",
        params![
            id,
            input.source_id,
            locator_s,
            input.anchor_kind,
            anchor.to_string(),
            input.body,
            input.color,
            stack,
            now.clone(),
            now,
        ],
    )?;
    get_one(&db, &id)
}

pub fn update(projects_root: &Path, project_id: &str, input: &NoteUpdate) -> AppResult<Note> {
    let db = projects::open_db(projects_root, project_id)?;
    let current = get_one(&db, &input.note_id)?;
    if let Some(expected) = &input.expected_updated_at {
        if expected != &current.updated_at {
            return Err(AppError::new(
                "NOTE_CONFLICT",
                "error.note.conflict",
                "note was modified elsewhere",
            )
            .with_details(serde_json::json!({ "updatedAt": current.updated_at })));
        }
    }
    if current.deleted_at.is_some() {
        return Err(AppError::new(
            "NOTE_DELETED",
            "error.note.deleted",
            "note is deleted",
        ));
    }
    let tx = db.unchecked_transaction()?;
    if let Some(loc) = &input.locator {
        tx.execute(
            "UPDATE notes SET locator = ?2 WHERE id = ?1",
            params![input.note_id, loc.to_string()],
        )?;
    }
    if let Some(a) = &input.anchor_json {
        validate_anchor(&current.anchor_kind, a)?;
        tx.execute(
            "UPDATE notes SET anchor_json = ?2 WHERE id = ?1",
            params![input.note_id, a.to_string()],
        )?;
    }
    if let Some(body) = &input.body {
        validate_body(body)?;
        tx.execute(
            "UPDATE notes SET body = ?2 WHERE id = ?1",
            params![input.note_id, body],
        )?;
    }
    if let Some(color) = &input.color {
        validate_color(color)?;
        tx.execute(
            "UPDATE notes SET color = ?2 WHERE id = ?1",
            params![input.note_id, color],
        )?;
    }
    if let Some(order) = input.stack_order {
        tx.execute(
            "UPDATE notes SET stack_order = ?2 WHERE id = ?1",
            params![input.note_id, order],
        )?;
    }
    tx.execute(
        "UPDATE notes SET updated_at = ?2 WHERE id = ?1",
        params![input.note_id, now_iso8601()],
    )?;
    tx.commit()?;
    get_one(&db, &input.note_id)
}

pub fn delete(projects_root: &Path, project_id: &str, note_id: &str) -> AppResult<Note> {
    let db = projects::open_db(projects_root, project_id)?;
    let current = get_one(&db, note_id)?;
    if current.deleted_at.is_some() {
        return Ok(current);
    }
    db.execute(
        "UPDATE notes SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![note_id, now_iso8601()],
    )?;
    get_one(&db, note_id)
}

pub fn restore(projects_root: &Path, project_id: &str, note_id: &str) -> AppResult<Note> {
    let db = projects::open_db(projects_root, project_id)?;
    // The parent source may have been deleted (cascade removes the row); in
    // that case the row is gone and get_one already 404s.
    let _ = get_one(&db, note_id)?;
    db.execute(
        "UPDATE notes SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![note_id, now_iso8601()],
    )?;
    get_one(&db, note_id)
}

fn get_one(db: &Connection, note_id: &str) -> AppResult<Note> {
    db.query_row("SELECT * FROM notes WHERE id = ?1", [note_id], row_to_note)
        .optional()?
        .ok_or_else(|| AppError::new("NOTE_NOT_FOUND", "error.note.notFound", note_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;

    fn setup() -> (tempfile::TempDir, std::path::PathBuf, String) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("projects");
        std::fs::create_dir_all(&root).unwrap();
        let pid = "p1".to_string();
        let dir = root.join(&pid);
        for sub in ["sources", "derived", "workspace"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let db_path = dir.join("project.db");
        let conn = storage::open(&db_path).unwrap();
        storage::migrate::run(
            &conn,
            storage::PROJECT_MIGRATIONS,
            storage::PROJECT_SCHEMA_VERSION,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sources (id, kind, original_name, rel_path, status, added_at) VALUES ('s1','pdf','a.pdf','sources/s1/a.pdf','ready','2026-01-01')",
            [],
        )
        .unwrap();
        drop(conn);
        // register project dir mapping via projects service layout
        let _ = &pid;
        (tmp, root, pid)
    }

    #[test]
    fn crud_roundtrip_with_validation_and_conflict() {
        let (_tmp, root, pid) = setup();
        // projects::open_db expects <root>/<id>/project.db; our setup matches.
        let created = create(
            &root,
            &pid,
            &NoteCreate {
                project_id: pid.clone(),
                source_id: "s1".into(),
                locator: Some(serde_json::json!({"page": 3})),
                anchor_kind: "page".into(),
                anchor_json: Some(serde_json::json!({"page": 3, "x": 0.25, "y": 0.5})),
                body: "hello".into(),
                color: "yellow".into(),
            },
        )
        .unwrap();
        assert_eq!(created.body, "hello");
        let listed = list(&root, &pid, "s1").unwrap();
        assert_eq!(listed.len(), 1);

        // invalid anchor rejected
        assert!(create(
            &root,
            &pid,
            &NoteCreate {
                project_id: pid.clone(),
                source_id: "s1".into(),
                locator: None,
                anchor_kind: "page".into(),
                anchor_json: Some(serde_json::json!({"x": 9.0})),
                body: "x".into(),
                color: "yellow".into(),
            },
        )
        .is_err());

        // body too long rejected
        assert!(create(
            &root,
            &pid,
            &NoteCreate {
                project_id: pid.clone(),
                source_id: "s1".into(),
                locator: None,
                anchor_kind: "page".into(),
                anchor_json: None,
                body: "a".repeat(NOTE_BODY_MAX + 1),
                color: "yellow".into(),
            },
        )
        .is_err());

        // conflict detection
        let stale = "2000-01-01T00:00:00Z".to_string();
        let err = update(
            &root,
            &pid,
            &NoteUpdate {
                project_id: pid.clone(),
                note_id: created.id.clone(),
                locator: None,
                anchor_json: None,
                body: Some("new".into()),
                color: None,
                stack_order: None,
                expected_updated_at: Some(stale),
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "NOTE_CONFLICT");

        // delete + restore
        delete(&root, &pid, &created.id).unwrap();
        assert!(list(&root, &pid, "s1").unwrap().is_empty());
        restore(&root, &pid, &created.id).unwrap();
        assert_eq!(list(&root, &pid, "s1").unwrap().len(), 1);
    }
}
