//! Ingest pipeline (docs/04). Phase 1 covers the AI-free static formats:
//! text / markdown / json / jsonl / code and csv / tsv. PDF, images, office and
//! web links land in a follow-up commit; audio/video in Phase 5; Vision analysis
//! in Phase 4.

mod sheet;
mod text;

use crate::domain::source::SourceKind;
use crate::error::{AppError, AppResult};
use crate::services::chunk;
use crate::services::retrieval::cjk_bigram;
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// One parsed unit before it becomes a `documents` row.
pub struct Unit {
    pub ordinal: u32,
    pub kind: &'static str,
    pub title: Option<String>,
    pub text: String,
    pub locator: serde_json::Value,
}

pub struct IngestCtx<'a> {
    pub project_db: &'a Connection,
    pub app_db: &'a Connection,
    pub project_id: &'a str,
    pub source_id: &'a str,
    pub source_name: &'a str,
    pub project_dir: &'a Path,
}

pub struct Outcome {
    pub documents: u32,
    pub chunks: u32,
    pub lang: Option<String>,
    /// true if some non-fatal step (AI) was skipped — caller sets ready_partial.
    pub partial: bool,
}

pub fn derived_dir(project_dir: &Path, source_id: &str) -> PathBuf {
    project_dir.join("derived").join(source_id)
}

/// Run the whole pipeline for one source. Idempotent: wipes `derived/<sid>/`
/// and the source's documents/chunks first (docs/04 §0).
pub fn run(ctx: &IngestCtx, kind: SourceKind, abs_path: &Path) -> AppResult<Outcome> {
    let dd = derived_dir(ctx.project_dir, ctx.source_id);
    if dd.exists() {
        std::fs::remove_dir_all(&dd)?;
    }
    std::fs::create_dir_all(&dd)?;
    ctx.project_db
        .execute("DELETE FROM documents WHERE source_id = ?1", [ctx.source_id])?;
    ctx.app_db.execute(
        "DELETE FROM global_index WHERE source_id = ?1",
        [ctx.source_id],
    )?;

    let units = match kind {
        SourceKind::Text | SourceKind::Markdown | SourceKind::Code => text::parse_text(kind, abs_path)?,
        SourceKind::Json => text::parse_json(abs_path)?,
        SourceKind::Jsonl => text::parse_jsonl(abs_path)?,
        SourceKind::Sheet => sheet::parse_sheet(abs_path)?,
        other => {
            return Err(AppError::new(
                "SOURCE_UNSUPPORTED_FORMAT",
                "error.source.unsupported",
                format!("ingest for {other:?} is not implemented yet"),
            ))
        }
    };

    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "no readable content",
        ));
    }

    // derived/<sid>/document.md — the human+AI readable canonical form (docs/04 §0 ⑤)
    let mut doc_md = String::new();
    for u in &units {
        if let Some(t) = &u.title {
            doc_md.push_str(&format!("\n\n## {t}\n\n"));
        } else {
            doc_md.push_str("\n\n");
        }
        doc_md.push_str(&u.text);
    }
    std::fs::write(dd.join("document.md"), doc_md.trim_start())?;

    let lang = detect_lang(&units);
    let now = now_iso8601();
    let mut total_chunks = 0u32;

    for u in &units {
        let doc_id = Uuid::now_v7().to_string();
        ctx.project_db.execute(
            "INSERT INTO documents (id, source_id, ordinal, kind, title, text, locator)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                doc_id,
                ctx.source_id,
                u.ordinal,
                u.kind,
                u.title,
                u.text,
                u.locator.to_string()
            ],
        )?;

        for (i, piece) in chunk::split(&u.text).into_iter().enumerate() {
            let header = provenance_header(ctx.source_name, u);
            let body = format!("{header}\n{}", piece.text);
            let chunk_id = Uuid::now_v7().to_string();
            let locator = serde_json::json!({ "t": "line", "start": piece.start, "end": piece.end });
            ctx.project_db.execute(
                "INSERT INTO chunks
                   (id, source_id, document_id, ordinal, text, text_bigram, tokens, locator, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    chunk_id,
                    ctx.source_id,
                    doc_id,
                    i as i64,
                    body,
                    cjk_bigram(&body),
                    piece.tokens as i64,
                    locator.to_string(),
                    now
                ],
            )?;
            total_chunks += 1;
        }
    }

    // Cross-project mirror (FR-N3): one row per source (title) + one per document.
    ctx.app_db.execute(
        "INSERT INTO global_index (project_id, source_id, kind, ref_id, title, body, body_raw)
         VALUES (?1, ?2, 'source', ?2, ?3, ?4, ?3)",
        params![
            ctx.project_id,
            ctx.source_id,
            ctx.source_name,
            cjk_bigram(ctx.source_name)
        ],
    )?;
    for u in &units {
        ctx.app_db.execute(
            "INSERT INTO global_index (project_id, source_id, kind, ref_id, title, body, body_raw)
             VALUES (?1, ?2, 'chunk', ?3, ?4, ?5, ?6)",
            params![
                ctx.project_id,
                ctx.source_id,
                format!("{}#{}", ctx.source_id, u.ordinal),
                u.title.clone().unwrap_or_default(),
                cjk_bigram(&u.text),
                u.text
            ],
        )?;
    }

    Ok(Outcome {
        documents: units.len() as u32,
        chunks: total_chunks,
        lang,
        partial: false,
    })
}

fn provenance_header(source_name: &str, u: &Unit) -> String {
    match &u.title {
        Some(t) => format!("《{source_name} / {} {} / {t}》", u.kind, u.ordinal),
        None => format!("《{source_name} / {} {}》", u.kind, u.ordinal),
    }
}

/// Cheap language guess from the first units — Japanese/Chinese vs. English.
fn detect_lang(units: &[Unit]) -> Option<String> {
    let sample: String = units.iter().map(|u| u.text.as_str()).take(3).collect();
    if sample.trim().is_empty() {
        return None;
    }
    let (mut cjk, mut latin) = (0usize, 0usize);
    for c in sample.chars().take(2000) {
        let u = c as u32;
        if (0x3040..=0x30FF).contains(&u) {
            return Some("ja".into());
        }
        if (0x4E00..=0x9FFF).contains(&u) {
            cjk += 1;
        } else if c.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    if cjk > latin {
        Some("zh".into())
    } else if latin > 0 {
        Some("en".into())
    } else {
        None
    }
}
