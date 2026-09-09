//! Project export / import (docs/03 §6). A `.wakaru.zip` is a self-contained
//! folder — `manifest.json` (with `schemaVersion`) + `project.db` + `sources/` +
//! `derived/` + `workspace/` + `README.txt`. Embeddings are rebuilt on import
//! unless explicitly bundled (§6.2).

use crate::domain::export::*;
use crate::error::{AppError, AppResult};
use crate::services::projects;
use crate::storage::{self, migrate::now_iso8601, PROJECT_SCHEMA_VERSION};
use rusqlite::params;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

const SKIP_DIRS: [&str; 1] = ["thumbs"];
const MAX_IMPORT_FILES: usize = 100_000;
const MAX_IMPORT_BYTES: u64 = 4 * 1024 * 1024 * 1024;

struct ImportDirGuard {
    path: std::path::PathBuf,
    keep: bool,
}

impl Drop for ImportDirGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

pub fn estimate(projects_root: &Path, project_ids: &[String]) -> AppResult<Vec<ExportEstimate>> {
    let mut out = Vec::new();
    for id in project_ids {
        let dir = projects::project_dir(projects_root, id);
        let (mut base, mut emb) = (0u64, 0u64);
        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&dir).unwrap_or(entry.path());
            if rel
                .components()
                .any(|c| SKIP_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()))
            {
                continue;
            }
            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            base += len;
            emb += len;
        }
        // rough embeddings size: 384 f32 * chunk count, if we can read the db
        if let Ok(conn) = storage::open(&projects::project_db_path(projects_root, id)) {
            let chunks: i64 = conn
                .query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
                .unwrap_or(0);
            emb += (chunks as u64) * 384 * 4;
        }
        out.push(ExportEstimate {
            project_id: id.clone(),
            bytes_without_embeddings: base,
            bytes_with_embeddings: emb,
        });
    }
    Ok(out)
}

pub fn export(
    app_db: &rusqlite::Connection,
    projects_root: &Path,
    input: &ExportInput,
) -> AppResult<ExportResult> {
    std::fs::create_dir_all(&input.dest_dir)?;
    let mut files = Vec::new();

    for id in &input.ids {
        let project = projects::get(app_db, id)?;
        let dir = projects::project_dir(projects_root, id);

        let manifest = build_manifest(app_db, projects_root, &project)?;
        let safe_name = sanitize(&project.name);
        let out_path = format!(
            "{}/{safe_name}.wakaru.zip",
            input.dest_dir.trim_end_matches('/')
        );

        let file = File::create(&out_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("manifest.json", opts)?;
        zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

        zip.start_file("README.txt", opts)?;
        zip.write_all(readme_txt(&project.name).as_bytes())?;

        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&dir).unwrap_or(entry.path());
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if rel_str == "manifest.json"
                || SKIP_DIRS
                    .iter()
                    .any(|d| rel_str.starts_with(&format!("{d}/")))
            {
                continue;
            }
            if !input.include_embeddings && rel_str == "embeddings.bin" {
                continue;
            }
            zip.start_file(rel_str, opts)?;
            let mut f = File::open(entry.path())?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
        }

        zip.finish()?;
        tracing::info!(project = %id, path = %out_path, "exported");
        files.push(out_path);
    }

    Ok(ExportResult { files })
}

pub fn import(
    app_db: &rusqlite::Connection,
    projects_root: &Path,
    zip_path: &str,
) -> AppResult<crate::domain::project::Project> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::new("IMPORT_BAD_ZIP", "error.import.badZip", e.to_string()))?;
    if archive.len() > MAX_IMPORT_FILES {
        return Err(AppError::new(
            "IMPORT_TOO_LARGE",
            "error.import.badArchive",
            "archive contains too many files",
        ));
    }

    // Read manifest first for the version check.
    let manifest: serde_json::Value = {
        let mut s = String::new();
        archive
            .by_name("manifest.json")
            .map_err(|_| {
                AppError::new(
                    "IMPORT_NO_MANIFEST",
                    "error.import.noManifest",
                    "no manifest.json",
                )
            })?
            .read_to_string(&mut s)?;
        serde_json::from_str(&s)?
    };
    let stored_ver = manifest
        .get("schemaVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");

    match storage::migrate::version_verdict(stored_ver, PROJECT_SCHEMA_VERSION) {
        VersionVerdict::Reject => {
            return Err(AppError::new(
                "IMPORT_TOO_NEW",
                "error.import.tooNew",
                format!("made with a newer WAKARU (schema {stored_ver})"),
            )
            .with_details(serde_json::json!({ "schemaVersion": stored_ver })));
        }
        VersionVerdict::WarnOpen => {
            tracing::warn!(
                schema = stored_ver,
                "importing a newer-minor project; unknown columns preserved"
            );
        }
        _ => {}
    }

    // New id — never collide with an existing project (§6.3).
    let new_id = Uuid::now_v7().to_string();
    let dir = projects::project_dir(projects_root, &new_id);
    std::fs::create_dir_all(&dir)?;
    let mut dir_guard = ImportDirGuard {
        path: dir.clone(),
        keep: false,
    };
    for sub in projects::SUBDIRS {
        std::fs::create_dir_all(dir.join(sub))?;
    }

    let mut extracted_bytes = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            AppError::new(
                "IMPORT_BAD_ARCHIVE",
                "error.import.badArchive",
                format!("cannot read zip entry {i}: {e}"),
            )
        })?;
        let enclosed = entry.enclosed_name().ok_or_else(|| {
            AppError::new(
                "IMPORT_BAD_ARCHIVE",
                "error.import.badArchive",
                format!("unsafe archive entry at index {i}"),
            )
        })?;
        let name = enclosed.to_string_lossy().replace('\\', "/");
        if name == "manifest.json" || name == "README.txt" || entry.is_dir() {
            continue;
        }
        extracted_bytes = extracted_bytes.saturating_add(entry.size());
        if extracted_bytes > MAX_IMPORT_BYTES {
            return Err(AppError::new(
                "IMPORT_TOO_LARGE",
                "error.import.badArchive",
                "archive expands beyond the project import limit",
            ));
        }
        let dest = dir.join(enclosed);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        std::fs::write(&dest, buf)?;
    }

    // Run project migrations over the imported db (handles Migrate verdict).
    let conn = storage::open_project_db(&projects::project_db_path(projects_root, &new_id))?;
    let source_count: i64 = conn
        .query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
        .unwrap_or(0);

    // If embeddings weren't bundled, drop any stale vector table — it will be
    // rebuilt on next ingest / search (§6.2).
    if !dir.join("embeddings.bin").exists() {
        let _ =
            conn.execute_batch("DROP TABLE IF EXISTS chunk_vectors; DELETE FROM embedding_meta;");
    }

    let name = manifest
        .pointer("/project/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported project")
        .to_string();
    let now = now_iso8601();
    let sort_order: i32 = app_db
        .query_row(
            "SELECT COALESCE(MAX(sort_order),0)+1 FROM projects",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let tx = app_db.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO projects (id, name, description, color, dir_name, schema_version, created_at, updated_at, sort_order)
         VALUES (?1, ?2, ?3, 'accent-1', ?1, ?4, ?5, ?5, ?6)",
        params![
            new_id,
            name,
            manifest.pointer("/project/description").and_then(|v| v.as_str()).unwrap_or(""),
            PROJECT_SCHEMA_VERSION,
            now,
            sort_order
        ],
    )?;

    // Rebuild the cross-project FTS mirror from the imported chunks.
    rebuild_global_index(&tx, &conn, &new_id)?;
    tx.commit()?;
    dir_guard.keep = true;

    tracing::info!(project = %new_id, sources = source_count, "imported");
    projects::get(app_db, &new_id)
}

fn rebuild_global_index(
    app_db: &rusqlite::Connection,
    project_db: &rusqlite::Connection,
    project_id: &str,
) -> AppResult<()> {
    app_db.execute(
        "DELETE FROM global_index WHERE project_id = ?1",
        [project_id],
    )?;
    let mut stmt = project_db.prepare(
        "SELECT s.id, s.original_name, d.ordinal, d.title, d.text
         FROM documents d JOIN sources s ON s.id = d.source_id",
    )?;
    let rows: Vec<(String, String, i64, Option<String>, String)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);
    for (sid, sname, ordinal, title, text) in rows {
        app_db.execute(
            "INSERT INTO global_index (project_id, source_id, kind, ref_id, title, body, body_raw)
             VALUES (?1, ?2, 'chunk', ?3, ?4, ?5, ?6)",
            params![
                project_id,
                sid,
                format!("{sid}#{ordinal}"),
                title.unwrap_or_default(),
                crate::services::retrieval::cjk_bigram(&text),
                text
            ],
        )?;
        let _ = sname;
    }
    Ok(())
}

fn build_manifest(
    app_db: &rusqlite::Connection,
    projects_root: &Path,
    project: &crate::domain::project::Project,
) -> AppResult<serde_json::Value> {
    let conn = storage::open(&projects::project_db_path(projects_root, &project.id))?;
    let sources: i64 = conn
        .query_row("SELECT count(*) FROM sources", [], |r| r.get(0))
        .unwrap_or(0);
    let documents: i64 = conn
        .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
        .unwrap_or(0);
    let chunks: i64 = conn
        .query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
        .unwrap_or(0);
    let embed: Option<(String, i64)> = conn
        .query_row(
            "SELECT model, dim FROM embedding_meta WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let _ = app_db;
    Ok(serde_json::json!({
        "schemaVersion": PROJECT_SCHEMA_VERSION,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "project": {
            "id": project.id,
            "name": project.name,
            "description": project.description,
            "createdAt": project.created_at,
            "updatedAt": project.updated_at,
        },
        "counts": { "sources": sources, "documents": documents, "chunks": chunks },
        "embedding": embed.map(|(m, d)| serde_json::json!({ "model": m, "dim": d })),
        "exportedAt": now_iso8601(),
    }))
}

fn readme_txt(name: &str) -> String {
    format!(
        "This is a WAKARU project export.\n\nProject: {name}\n\nTo restore it, open WAKARU and use\nSettings -> Project management -> Import (ZIP).\n\nhttps://studio-rizi.pages.dev/projects/wakaru/\n"
    )
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.trim_matches('_').is_empty() {
        "project".into()
    } else {
        s
    }
}
