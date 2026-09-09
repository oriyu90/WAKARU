//! Project lifecycle (docs/03 §2). A project is one self-contained folder:
//! `projects/<id>/{manifest.json, project.db, sources/, derived/, workspace/,
//! thumbs/, exports/}`.

use crate::domain::project::*;
use crate::domain::ProjectColor;
use crate::error::{AppError, AppResult};
use crate::storage::{self, migrate::now_iso8601};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const SUBDIRS: [&str; 5] = ["sources", "derived", "workspace", "thumbs", "exports"];

pub fn project_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

pub fn project_db_path(root: &Path, id: &str) -> PathBuf {
    project_dir(root, id).join("project.db")
}

pub fn open_db(root: &Path, id: &str) -> AppResult<Connection> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(AppError::new(
            "PROJECT_NOT_FOUND",
            "error.project.notFound",
            "invalid project id",
        ));
    }
    let path = project_db_path(root, id);
    if !path.exists() {
        return Err(AppError::new(
            "PROJECT_NOT_FOUND",
            "error.project.notFound",
            format!("no project.db for {id}"),
        ));
    }
    storage::open_project_db(&path)
}

pub fn create(app_db: &Connection, root: &Path, input: CreateProjectInput) -> AppResult<Project> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "PROJECT_NAME_REQUIRED",
            "error.project.nameRequired",
            "project name is required",
        ));
    }
    let id = Uuid::now_v7().to_string();
    let now = now_iso8601();
    let color = input.color.unwrap_or_default();
    let description = input.description.unwrap_or_default();

    let dir = project_dir(root, &id);
    std::fs::create_dir_all(&dir)?;
    for sub in SUBDIRS {
        std::fs::create_dir_all(dir.join(sub))?;
    }

    // project.db + migrations
    let _conn = storage::open_project_db(&project_db_path(root, &id))?;

    let sort_order: i32 = app_db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM projects",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1);

    app_db.execute(
        "INSERT INTO projects
           (id, name, description, color, dir_name, schema_version, created_at, updated_at, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)",
        params![
            id,
            name,
            description,
            color_to_str(color),
            id,
            PROJECT_SCHEMA_VERSION,
            now,
            sort_order
        ],
    )?;

    write_manifest(root, &id, name, &description, color, &now)?;
    tracing::info!(project = %id, "created project");

    Ok(Project {
        id,
        name: name.to_string(),
        description,
        color,
        schema_version: PROJECT_SCHEMA_VERSION.to_string(),
        created_at: now.clone(),
        updated_at: now,
        opened_at: None,
        archived_at: None,
        sort_order,
    })
}

pub fn list(
    app_db: &Connection,
    root: &Path,
    include_archived: bool,
) -> AppResult<Vec<ProjectSummary>> {
    let sql = if include_archived {
        "SELECT id, name, description, color, archived_at, opened_at, updated_at
         FROM projects ORDER BY sort_order"
    } else {
        "SELECT id, name, description, color, archived_at, opened_at, updated_at
         FROM projects WHERE archived_at IS NULL ORDER BY sort_order"
    };
    let mut stmt = app_db.prepare(sql)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, name, description, color, archived_at, opened_at, updated_at) in rows {
        let source_count = count_sources(root, &id).unwrap_or(0);
        out.push(ProjectSummary {
            id,
            name,
            description,
            color: color_from_str(&color),
            archived: archived_at.is_some(),
            source_count,
            last_opened_at: opened_at,
            updated_at,
        });
    }
    Ok(out)
}

pub fn get(app_db: &Connection, id: &str) -> AppResult<Project> {
    app_db
        .query_row(
            "SELECT id, name, description, color, schema_version, created_at, updated_at,
                    opened_at, archived_at, sort_order
             FROM projects WHERE id = ?1",
            [id],
            |r| {
                Ok(Project {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    color: color_from_str(&r.get::<_, String>(3)?),
                    schema_version: r.get(4)?,
                    created_at: r.get(5)?,
                    updated_at: r.get(6)?,
                    opened_at: r.get(7)?,
                    archived_at: r.get(8)?,
                    sort_order: r.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "error.project.notFound", id))
}

pub fn update(app_db: &Connection, root: &Path, input: UpdateProjectInput) -> AppResult<Project> {
    let cur = get(app_db, &input.id)?;
    let name = input.name.unwrap_or(cur.name);
    let description = input.description.unwrap_or(cur.description);
    let color = input.color.unwrap_or(cur.color);
    let now = now_iso8601();
    app_db.execute(
        "UPDATE projects SET name=?2, description=?3, color=?4, updated_at=?5 WHERE id=?1",
        params![input.id, name.trim(), description, color_to_str(color), now],
    )?;
    write_manifest(root, &input.id, name.trim(), &description, color, &now)?;
    get(app_db, &input.id)
}

pub fn set_archived(app_db: &Connection, id: &str, archived: bool) -> AppResult<()> {
    get(app_db, id)?; // 404 if missing
    let val = if archived { Some(now_iso8601()) } else { None };
    app_db.execute(
        "UPDATE projects SET archived_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, val, now_iso8601()],
    )?;
    Ok(())
}

pub fn mark_opened(app_db: &Connection, id: &str) -> AppResult<()> {
    app_db.execute(
        "UPDATE projects SET opened_at = ?2 WHERE id = ?1",
        params![id, now_iso8601()],
    )?;
    Ok(())
}

/// Delete requires the caller to re-type the project name (FR-P6). Order:
/// DB row first, then the folder (docs/03 §8).
pub fn delete(app_db: &Connection, root: &Path, id: &str, confirm_name: &str) -> AppResult<()> {
    let project = get(app_db, id)?;
    if confirm_name.trim() != project.name {
        return Err(AppError::new(
            "PROJECT_DELETE_NAME_MISMATCH",
            "error.project.deleteNameMismatch",
            "confirmation name does not match",
        ));
    }
    app_db.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    app_db.execute("DELETE FROM global_index WHERE project_id = ?1", [id])?;
    let dir = project_dir(root, id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            // Leave it for the startup orphan GC (docs/03 §8) rather than fail.
            tracing::warn!(project = %id, error = %e, "project folder removal failed; deferred to GC");
        }
    }
    tracing::info!(project = %id, "deleted project");
    Ok(())
}

/// Remove `projects/<dir>` folders that have no matching row (docs/03 §8).
pub fn gc_orphans(app_db: &Connection, root: &Path) -> AppResult<u32> {
    if !root.exists() {
        return Ok(0);
    }
    let mut known = std::collections::HashSet::new();
    let mut stmt = app_db.prepare("SELECT dir_name FROM projects")?;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        known.insert(row?);
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !known.contains(&name) {
            std::fs::remove_dir_all(entry.path())?;
            removed += 1;
            tracing::info!(dir = %name, "gc: removed orphan project folder");
        }
    }
    Ok(removed)
}

fn count_sources(root: &Path, id: &str) -> AppResult<u32> {
    let path = project_db_path(root, id);
    if !path.exists() {
        return Ok(0);
    }
    let conn = storage::open(&path)?;
    let n: i64 = conn
        .query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
        .unwrap_or(0);
    Ok(n as u32)
}

fn write_manifest(
    root: &Path,
    id: &str,
    name: &str,
    description: &str,
    color: ProjectColor,
    now: &str,
) -> AppResult<()> {
    let manifest = serde_json::json!({
        "schemaVersion": PROJECT_SCHEMA_VERSION,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "project": {
            "id": id,
            "name": name,
            "description": description,
            "color": color_to_str(color),
            "updatedAt": now,
        },
    });
    std::fs::write(
        project_dir(root, id).join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

fn color_to_str(c: ProjectColor) -> &'static str {
    match c {
        ProjectColor::Accent1 => "accent-1",
        ProjectColor::Accent2 => "accent-2",
        ProjectColor::Accent3 => "accent-3",
        ProjectColor::Accent4 => "accent-4",
        ProjectColor::Accent5 => "accent-5",
    }
}

fn color_from_str(s: &str) -> ProjectColor {
    match s {
        "accent-2" => ProjectColor::Accent2,
        "accent-3" => ProjectColor::Accent3,
        "accent-4" => ProjectColor::Accent4,
        "accent-5" => ProjectColor::Accent5,
        _ => ProjectColor::Accent1,
    }
}
