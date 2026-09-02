//! Viewer tab state (docs/06 §5.1). Tabs persist in `viewer_tabs` and are
//! restored when a project is opened (FR-V6). A source is only ever in one tab —
//! opening it again reuses that tab (AC-2-3).

use crate::domain::source::SourceKind;
use crate::domain::viewer::*;
use crate::error::{AppError, AppResult};
use crate::services::sources::kind_from_str;
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub fn get_tabs(project_db: &Connection) -> AppResult<Vec<ViewerTab>> {
    let mut stmt = project_db.prepare(
        "SELECT t.id, t.source_id, s.kind, s.original_name, t.locator, t.pinned, t.ordinal
         FROM viewer_tabs t JOIN sources s ON s.id = t.source_id
         ORDER BY t.pinned DESC, t.ordinal",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ViewerTab {
                id: r.get(0)?,
                source_id: r.get(1)?,
                kind: kind_from_str(&r.get::<_, String>(2)?),
                name: r.get(3)?,
                locator: serde_json::from_str(&r.get::<_, String>(4)?)
                    .unwrap_or(serde_json::json!({})),
                pinned: r.get::<_, i64>(5)? != 0,
                ordinal: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn open_tab(
    project_db: &Connection,
    source_id: &str,
    locator: Option<serde_json::Value>,
) -> AppResult<ViewerTab> {
    // Source must exist.
    let (kind, name): (String, String) = project_db
        .query_row(
            "SELECT kind, original_name FROM sources WHERE id = ?1",
            [source_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::new("SOURCE_NOT_FOUND", "error.source.notFound", source_id))?;

    let loc = locator.unwrap_or(serde_json::json!({}));

    if let Some(id) = existing_tab(project_db, source_id)? {
        project_db.execute(
            "UPDATE viewer_tabs SET locator = ?2 WHERE id = ?1",
            params![id, loc.to_string()],
        )?;
        return single(project_db, &id);
    }

    let id = Uuid::now_v7().to_string();
    let next: i32 = project_db
        .query_row(
            "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM viewer_tabs",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1);
    project_db.execute(
        "INSERT INTO viewer_tabs (id, source_id, locator, pinned, ordinal, opened_at)
         VALUES (?1, ?2, ?3, 0, ?4, ?5)",
        params![id, source_id, loc.to_string(), next, now_iso8601()],
    )?;

    Ok(ViewerTab {
        id,
        source_id: source_id.to_string(),
        kind: kind_from_str(&kind),
        name,
        locator: loc,
        pinned: false,
        ordinal: next,
    })
}

pub fn close_tab(project_db: &Connection, tab_id: &str) -> AppResult<()> {
    project_db.execute("DELETE FROM viewer_tabs WHERE id = ?1", [tab_id])?;
    Ok(())
}

pub fn update_locator(
    project_db: &Connection,
    tab_id: &str,
    locator: serde_json::Value,
) -> AppResult<()> {
    project_db.execute(
        "UPDATE viewer_tabs SET locator = ?2 WHERE id = ?1",
        params![tab_id, locator.to_string()],
    )?;
    Ok(())
}

pub fn pin_tab(project_db: &Connection, tab_id: &str, pinned: bool) -> AppResult<()> {
    project_db.execute(
        "UPDATE viewer_tabs SET pinned = ?2 WHERE id = ?1",
        params![tab_id, pinned as i64],
    )?;
    Ok(())
}

pub fn reorder_tabs(project_db: &Connection, tab_ids: &[String]) -> AppResult<()> {
    let tx = project_db.unchecked_transaction()?;
    for (i, id) in tab_ids.iter().enumerate() {
        tx.execute(
            "UPDATE viewer_tabs SET ordinal = ?2 WHERE id = ?1",
            params![id, i as i64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn existing_tab(project_db: &Connection, source_id: &str) -> AppResult<Option<String>> {
    Ok(project_db
        .query_row(
            "SELECT id FROM viewer_tabs WHERE source_id = ?1",
            [source_id],
            |r| r.get(0),
        )
        .optional()?)
}

fn single(project_db: &Connection, tab_id: &str) -> AppResult<ViewerTab> {
    project_db
        .query_row(
            "SELECT t.id, t.source_id, s.kind, s.original_name, t.locator, t.pinned, t.ordinal
             FROM viewer_tabs t JOIN sources s ON s.id = t.source_id WHERE t.id = ?1",
            [tab_id],
            |r| {
                Ok(ViewerTab {
                    id: r.get(0)?,
                    source_id: r.get(1)?,
                    kind: kind_from_str(&r.get::<_, String>(2)?),
                    name: r.get(3)?,
                    locator: serde_json::from_str(&r.get::<_, String>(4)?)
                        .unwrap_or(serde_json::json!({})),
                    pinned: r.get::<_, i64>(5)? != 0,
                    ordinal: r.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("TAB_NOT_FOUND", "error.viewer.tabNotFound", tab_id))
}

/// `source_get_document` — text + optional page image for one ordinal.
pub fn get_document(
    project_db: &Connection,
    projects_root: &std::path::Path,
    project_id: &str,
    source_id: &str,
    ordinal: u32,
) -> AppResult<DocumentPayload> {
    let total: u32 = project_db
        .query_row(
            "SELECT COUNT(*) FROM documents WHERE source_id = ?1",
            [source_id],
            |r| r.get::<_, i64>(0).map(|v| v as u32),
        )
        .unwrap_or(0);
    if total == 0 {
        return Err(AppError::new(
            "DOCUMENT_NOT_READY",
            "error.document.notReady",
            "source has no parsed documents yet",
        ));
    }
    let ordinal = ordinal.clamp(1, total);

    let (kind, title, text, image_rel): (String, Option<String>, String, Option<String>) =
        project_db
            .query_row(
                "SELECT kind, title, text, image_rel FROM documents
             WHERE source_id = ?1 AND ordinal = ?2",
                params![source_id, ordinal],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| {
                AppError::new(
                    "DOCUMENT_NOT_FOUND",
                    "error.document.notFound",
                    "ordinal out of range",
                )
            })?;

    let image_url = image_rel.map(|rel| {
        crate::services::assets::url(project_id, source_id, &format!("derived/{source_id}/{rel}"))
    });
    let _ = projects_root;

    Ok(DocumentPayload {
        source_id: source_id.to_string(),
        ordinal,
        total,
        kind,
        title,
        text,
        image_url,
        locator: serde_json::json!({ "t": "page", "page": ordinal }),
    })
}

pub fn source_detail(
    project_db: &Connection,
    project_id: &str,
    source_id: &str,
) -> AppResult<SourceDetail> {
    let s = crate::services::sources::get(project_db, source_id)?;
    let rel_path: Option<String> = project_db
        .query_row(
            "SELECT rel_path FROM sources WHERE id = ?1",
            [source_id],
            |r| r.get(0),
        )
        .optional()?;
    let primary = match s.kind {
        SourceKind::Image => Some(crate::services::assets::url(
            project_id,
            source_id,
            &format!("derived/{source_id}/pages/0001.png"),
        )),
        SourceKind::Weblink => Some(crate::services::assets::url(
            project_id,
            source_id,
            &format!("derived/{source_id}/reader.md"),
        )),
        // Every imported file is exposed through the project-scoped asset
        // protocol. The webview never receives the host's absolute path.
        _ => rel_path
            .as_deref()
            .map(|rel| crate::services::assets::url(project_id, source_id, rel)),
    };
    Ok(SourceDetail {
        id: s.id,
        kind: s.kind,
        name: s.original_name,
        url: s.url,
        page_count: s.page_count,
        status: s.status,
        mime: s.mime,
        bytes: s.bytes,
        primary_asset_url: primary,
        ocr_status: s.ocr_status,
    })
}
