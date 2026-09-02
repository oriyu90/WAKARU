//! Source add / list / delete / re-analyze (docs/04 §0). Adding copies the
//! original into `sources/<id>/`, hashes it, and queues an ingest job. The
//! original is never modified or deleted afterwards except on source/project
//! delete (I-3).

use crate::domain::source::*;
use crate::error::{AppError, AppResult};
use crate::jobs::JobRegistry;
use crate::services::ingest::{self, IngestCtx, IngestInput};
use crate::services::projects;
use crate::storage::{self, migrate::now_iso8601};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// Detect the source kind from extension, then confirm with magic bytes where it
/// matters (docs/04 §0 ① — do not trust the extension alone).
pub fn detect_kind(path: &Path) -> Option<SourceKind> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let by_ext = match ext.as_str() {
        "pdf" => Some(SourceKind::Pdf),
        "pptx" | "ppt" => Some(SourceKind::Slides),
        "docx" | "doc" => Some(SourceKind::Doc),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff" | "heic" | "heif"
        | "avif" => Some(SourceKind::Image),
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" | "opus" => Some(SourceKind::Audio),
        "mp4" | "mov" | "m4v" | "mkv" | "webm" => Some(SourceKind::Video),
        "xlsx" | "xls" | "csv" | "tsv" => Some(SourceKind::Sheet),
        "md" | "markdown" | "mdx" => Some(SourceKind::Markdown),
        "json" => Some(SourceKind::Json),
        "jsonl" | "ndjson" => Some(SourceKind::Jsonl),
        "txt" | "text" | "log" | "rtf" => Some(SourceKind::Text),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "c" | "h" | "cpp" | "hpp"
        | "cs" | "rb" | "php" | "swift" | "kt" | "sh" | "sql" | "toml" | "yaml" | "yml" | "css"
        | "html" | "xml" => Some(SourceKind::Code),
        _ => None,
    };

    // Magic-byte cross-check for the binary kinds.
    if let Ok(Some(t)) = infer::get_from_path(path) {
        let mime = t.mime_type();
        let magic = if mime.starts_with("image/") {
            Some(SourceKind::Image)
        } else if mime == "application/pdf" {
            Some(SourceKind::Pdf)
        } else if mime.starts_with("audio/") {
            Some(SourceKind::Audio)
        } else if mime.starts_with("video/") {
            Some(SourceKind::Video)
        } else {
            None
        };
        if let Some(m) = magic {
            return Some(m);
        }
    }

    by_ext
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn row_to_source(r: &rusqlite::Row) -> rusqlite::Result<Source> {
    Ok(Source {
        id: r.get("id")?,
        kind: kind_from_str(&r.get::<_, String>("kind")?),
        original_name: r.get("original_name")?,
        url: r.get("url")?,
        mime: r.get("mime")?,
        bytes: r.get::<_, i64>("bytes")? as u64,
        sha256: r.get("sha256")?,
        status: status_from_str(&r.get::<_, String>("status")?),
        error_code: r.get("error_code")?,
        error_message: r.get("error_message")?,
        lang: r.get("lang")?,
        page_count: r.get::<_, Option<i64>>("page_count")?.map(|v| v as u32),
        duration_ms: r.get::<_, Option<i64>>("duration_ms")?.map(|v| v as u64),
        summary: r.get("summary")?,
        added_at: r.get("added_at")?,
        analyzed_at: r.get("analyzed_at")?,
        ocr_status: r.get("ocr_status")?,
    })
}

pub fn list(project_db: &Connection) -> AppResult<Vec<Source>> {
    let mut stmt = project_db.prepare("SELECT * FROM sources ORDER BY added_at DESC")?;
    let rows = stmt
        .query_map([], row_to_source)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(project_db: &Connection, source_id: &str) -> AppResult<Source> {
    project_db
        .query_row(
            "SELECT * FROM sources WHERE id = ?1",
            [source_id],
            row_to_source,
        )
        .optional()?
        .ok_or_else(|| AppError::new("SOURCE_NOT_FOUND", "error.source.notFound", source_id))
}

/// Insert queued rows for each path, then spawn ingest jobs. Returns immediately.
pub fn add_files(
    app: &AppHandle,
    app_db_path: &Path,
    projects_root: &Path,
    jobs: Arc<JobRegistry>,
    project_id: &str,
    paths: Vec<String>,
) -> AppResult<Vec<Source>> {
    let project_db = projects::open_db(projects_root, project_id)?;
    let mut created = Vec::new();

    for raw in paths {
        let src_path = PathBuf::from(&raw);
        if !src_path.is_file() {
            continue;
        }
        let added = add_one(&project_db, projects_root, project_id, &src_path)?;
        emit_status(app, project_id, &added.source);
        if let Some(kind) = added.kind {
            spawn_ingest(
                app.clone(),
                app_db_path.to_path_buf(),
                projects_root.to_path_buf(),
                jobs.clone(),
                project_id.to_string(),
                added.source.id.clone(),
                kind,
                IngestInput::File(added.dest),
            );
        }
        created.push(added.source);
    }

    Ok(created)
}

/// Add a web link as a source (docs/04 §8). The row is created immediately;
/// fetch + parse happen on the ingest job.
pub fn add_url(
    app: &AppHandle,
    app_db_path: &Path,
    projects_root: &Path,
    jobs: Arc<JobRegistry>,
    project_id: &str,
    url: &str,
) -> AppResult<Source> {
    let url = url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::new(
            "WEB_BLOCKED",
            "error.web.blocked",
            "only http(s) URLs can be added",
        ));
    }
    let project_db = projects::open_db(projects_root, project_id)?;
    let id = Uuid::now_v7().to_string();
    project_db.execute(
        "INSERT INTO sources (id, kind, original_name, rel_path, url, bytes, status, added_at)
         VALUES (?1, 'weblink', ?2, '', ?3, 0, 'queued', ?4)",
        params![id, url, url, now_iso8601()],
    )?;
    let source = get(&project_db, &id)?;
    emit_status(app, project_id, &source);
    spawn_ingest(
        app.clone(),
        app_db_path.to_path_buf(),
        projects_root.to_path_buf(),
        jobs,
        project_id.to_string(),
        id,
        SourceKind::Weblink,
        IngestInput::Url(url.to_string()),
    );
    Ok(source)
}

#[derive(Debug)]
pub struct Added {
    pub source: Source,
    pub kind: Option<SourceKind>,
    pub dest: PathBuf,
}

/// Copy one file into `sources/<id>/`, hash it, reject a duplicate, and insert
/// the row (`queued`, or `failed` for an unknown format). No events, no job —
/// the caller drives ingest. This is the unit the integration tests use.
pub fn add_one(
    project_db: &Connection,
    projects_root: &Path,
    project_id: &str,
    src_path: &Path,
) -> AppResult<Added> {
    let project_dir = projects::project_dir(projects_root, project_id);
    let original_name = src_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let bytes = std::fs::metadata(src_path).map(|m| m.len()).unwrap_or(0);
    let sha = sha256_file(src_path).ok();

    if let Some(ref s) = sha {
        let dup: Option<String> = project_db
            .query_row(
                "SELECT original_name FROM sources WHERE sha256 = ?1",
                [s],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = dup {
            return Err(AppError::new(
                "SOURCE_DUPLICATE",
                "error.source.duplicate",
                format!("{original_name} is already in this project as {existing}"),
            )
            .with_details(serde_json::json!({ "existing": existing, "name": original_name })));
        }
    }

    let id = Uuid::now_v7().to_string();
    let kind = detect_kind(src_path);
    let rel_path = format!("sources/{id}/{original_name}");
    let dest = project_dir.join(&rel_path);
    std::fs::create_dir_all(dest.parent().unwrap())?;
    std::fs::copy(src_path, &dest)?;

    let (status, err_code, err_msg) = match kind {
        Some(_) => ("queued", None, None),
        None => (
            "failed",
            Some("SOURCE_UNSUPPORTED_FORMAT".to_string()),
            Some(format!(
                "WAKARU does not support .{}",
                src_path.extension().and_then(|e| e.to_str()).unwrap_or("")
            )),
        ),
    };
    project_db.execute(
        "INSERT INTO sources
           (id, kind, original_name, rel_path, mime, bytes, sha256, status, error_code, error_message, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            kind.map(kind_to_str).unwrap_or("text"),
            original_name,
            rel_path,
            infer::get_from_path(src_path).ok().flatten().map(|t| t.mime_type().to_string()),
            bytes as i64,
            sha,
            status,
            err_code,
            err_msg,
            now_iso8601()
        ],
    )?;
    let source = get(project_db, &id)?;
    Ok(Added { source, kind, dest })
}

pub fn reanalyze(
    app: &AppHandle,
    app_db_path: &Path,
    projects_root: &Path,
    jobs: Arc<JobRegistry>,
    project_id: &str,
    source_id: &str,
) -> AppResult<()> {
    let project_db = projects::open_db(projects_root, project_id)?;
    let source = get(&project_db, source_id)?;
    let kind = detect_kind(
        &projects::project_dir(projects_root, project_id)
            .join(rel_path_of(&project_db, source_id)?),
    )
    .or(Some(source.kind));
    let Some(kind) = kind else {
        return Err(AppError::new(
            "SOURCE_UNSUPPORTED_FORMAT",
            "error.source.unsupported",
            "unknown kind",
        ));
    };
    project_db.execute(
        "UPDATE sources SET status='queued', error_code=NULL, error_message=NULL WHERE id=?1",
        [source_id],
    )?;
    let input = if source.kind == SourceKind::Weblink {
        IngestInput::Url(source.url.clone().unwrap_or_default())
    } else {
        IngestInput::File(
            projects::project_dir(projects_root, project_id)
                .join(rel_path_of(&project_db, source_id)?),
        )
    };
    spawn_ingest(
        app.clone(),
        app_db_path.to_path_buf(),
        projects_root.to_path_buf(),
        jobs,
        project_id.to_string(),
        source_id.to_string(),
        kind,
        input,
    );
    Ok(())
}

/// Delete a source and everything derived from it (docs/03 §8). DB rows first,
/// then files.
pub fn delete(
    app_db: &Connection,
    projects_root: &Path,
    project_id: &str,
    source_id: &str,
) -> AppResult<()> {
    let project_db = projects::open_db(projects_root, project_id)?;
    get(&project_db, source_id)?; // 404 if missing
                                  // documents / chunks / chunks_fts / threads / illustrations / viewer_tabs
                                  // all cascade from `sources` via ON DELETE CASCADE + the fts trigger.
    project_db.execute("DELETE FROM sources WHERE id = ?1", [source_id])?;
    app_db.execute(
        "DELETE FROM global_index WHERE project_id = ?1 AND source_id = ?2",
        params![project_id, source_id],
    )?;

    let project_dir = projects::project_dir(projects_root, project_id);
    for sub in ["sources", "derived", "thumbs"] {
        let p = project_dir.join(sub).join(source_id);
        if p.exists() {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
    Ok(())
}

fn rel_path_of(project_db: &Connection, source_id: &str) -> AppResult<String> {
    Ok(project_db.query_row(
        "SELECT rel_path FROM sources WHERE id = ?1",
        [source_id],
        |r| r.get(0),
    )?)
}

#[allow(clippy::too_many_arguments)]
fn spawn_ingest(
    app: AppHandle,
    app_db_path: PathBuf,
    projects_root: PathBuf,
    jobs: Arc<JobRegistry>,
    project_id: String,
    source_id: String,
    kind: SourceKind,
    input: IngestInput,
) {
    use crate::domain::JobKind;
    let (job_id, token) = jobs.create(
        JobKind::Ingest,
        Some(project_id.clone()),
        Some(source_id.clone()),
    );
    tauri::async_runtime::spawn_blocking(move || {
        jobs.mark_running(&job_id);
        let result = (|| -> AppResult<()> {
            let project_db = projects::open_db(&projects_root, &project_id)?;
            let app_db = storage::open(&app_db_path)?;
            set_status(
                &app,
                &project_db,
                &project_id,
                &source_id,
                "analyzing",
                None,
                None,
            );

            if token.is_cancelled() {
                set_status(
                    &app,
                    &project_db,
                    &project_id,
                    &source_id,
                    "queued",
                    None,
                    None,
                );
                return Ok(());
            }

            let name: String = project_db.query_row(
                "SELECT original_name FROM sources WHERE id = ?1",
                [&source_id],
                |r| r.get(0),
            )?;
            let ctx = IngestCtx {
                project_db: &project_db,
                app_db: &app_db,
                project_id: &project_id,
                source_id: &source_id,
                source_name: &name,
                project_dir: &projects::project_dir(&projects_root, &project_id),
            };
            let outcome = ingest::run(&ctx, kind, &input)?;
            let visual_complete = if kind == SourceKind::Image && outcome.partial {
                match tauri::async_runtime::block_on(
                    crate::services::vision::analyze_normalized_image(
                        &app_db,
                        &project_db,
                        &projects::project_dir(&projects_root, &project_id),
                        &project_id,
                        &source_id,
                    ),
                ) {
                    Ok(done) => done,
                    Err(error) => {
                        tracing::warn!(source = %source_id, error = %error, "visual analysis skipped");
                        false
                    }
                }
            } else {
                !outcome.partial
            };
            let partial = outcome.partial && !visual_complete;
            project_db.execute(
                "UPDATE sources
                   SET status=?2, lang=?3, page_count=?4, analyzed_at=?5,
                       original_name=COALESCE(?6, original_name),
                       ocr_status=?7,
                       error_code=NULL, error_message=NULL
                 WHERE id=?1",
                params![
                    source_id,
                    if partial { "ready_partial" } else { "ready" },
                    outcome.lang,
                    outcome.page_count.map(|v| v as i64),
                    now_iso8601(),
                    outcome.title_override,
                    outcome.ocr_status,
                ],
            )?;
            let final_status = if partial { "ready_partial" } else { "ready" };
            set_status(
                &app,
                &project_db,
                &project_id,
                &source_id,
                final_status,
                None,
                None,
            );
            tracing::info!(source = %source_id, docs = outcome.documents, chunks = outcome.chunks, "ingest done");

            // Vector index (docs/05 §2). No-op when no embedding endpoint is set —
            // search stays FTS-only (I-2).
            match tauri::async_runtime::block_on(crate::services::embed::index_source(
                &app_db,
                &project_db,
                &source_id,
                |_, _| {},
            )) {
                Ok(n) if n > 0 => {
                    tracing::info!(source = %source_id, embedded = n, "vector index updated")
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(source = %source_id, error = %e, "embedding failed (search stays FTS-only)")
                }
            }
            Ok(())
        })();

        if let Err(e) = result {
            if let Ok(project_db) = projects::open_db(&projects_root, &project_id) {
                let _ = project_db.execute(
                    "UPDATE sources SET status='failed', error_code=?2, error_message=?3 WHERE id=?1",
                    params![source_id, e.code, e.message],
                );
                set_status(
                    &app,
                    &project_db,
                    &project_id,
                    &source_id,
                    "failed",
                    Some(e.code.clone()),
                    None,
                );
            }
            jobs.finish(&job_id, crate::domain::JobStatus::Failed, Some(e.message));
        } else {
            jobs.finish(&job_id, crate::domain::JobStatus::Done, None);
        }
    });
}

fn set_status(
    app: &AppHandle,
    project_db: &Connection,
    project_id: &str,
    source_id: &str,
    status: &str,
    err_code: Option<String>,
    _err_msg: Option<String>,
) {
    let _ = project_db.execute(
        "UPDATE sources SET status = ?2 WHERE id = ?1 AND status != 'failed'",
        params![source_id, status],
    );
    if let Ok(source) = get(project_db, source_id) {
        emit_status(app, project_id, &source);
    } else {
        let _ = app.emit(
            "source://status",
            SourceStatusEvent {
                project_id: project_id.to_string(),
                source_id: source_id.to_string(),
                status: status_from_str(status),
                error_code: err_code,
            },
        );
    }
}

fn emit_status(app: &AppHandle, project_id: &str, source: &Source) {
    let _ = app.emit(
        "source://status",
        SourceStatusEvent {
            project_id: project_id.to_string(),
            source_id: source.id.clone(),
            status: source.status,
            error_code: source.error_code.clone(),
        },
    );
}

pub fn kind_to_str(k: SourceKind) -> &'static str {
    match k {
        SourceKind::Pdf => "pdf",
        SourceKind::Slides => "slides",
        SourceKind::Doc => "doc",
        SourceKind::Image => "image",
        SourceKind::Audio => "audio",
        SourceKind::Video => "video",
        SourceKind::Sheet => "sheet",
        SourceKind::Text => "text",
        SourceKind::Markdown => "markdown",
        SourceKind::Json => "json",
        SourceKind::Jsonl => "jsonl",
        SourceKind::Code => "code",
        SourceKind::Weblink => "weblink",
    }
}

pub fn kind_from_str(s: &str) -> SourceKind {
    match s {
        "pdf" => SourceKind::Pdf,
        "slides" => SourceKind::Slides,
        "doc" => SourceKind::Doc,
        "image" => SourceKind::Image,
        "audio" => SourceKind::Audio,
        "video" => SourceKind::Video,
        "sheet" => SourceKind::Sheet,
        "markdown" => SourceKind::Markdown,
        "json" => SourceKind::Json,
        "jsonl" => SourceKind::Jsonl,
        "code" => SourceKind::Code,
        "weblink" => SourceKind::Weblink,
        _ => SourceKind::Text,
    }
}

fn status_from_str(s: &str) -> SourceStatus {
    match s {
        "analyzing" => SourceStatus::Analyzing,
        "ready" => SourceStatus::Ready,
        "ready_partial" => SourceStatus::ReadyPartial,
        "failed" => SourceStatus::Failed,
        _ => SourceStatus::Queued,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_kind_by_extension() {
        assert_eq!(
            detect_kind(Path::new("a/b/notes.md")),
            Some(SourceKind::Markdown)
        );
        assert_eq!(detect_kind(Path::new("data.csv")), Some(SourceKind::Sheet));
        assert_eq!(detect_kind(Path::new("main.rs")), Some(SourceKind::Code));
        assert_eq!(detect_kind(Path::new("mystery.xyz")), None);
    }
}
