//! Ingest pipeline (docs/04): text / markdown / JSON / code, spreadsheets,
//! searchable PDF text, normalized images, Office documents, web links and
//! audio/video metadata/transcripts. Visual rendering is handled by the Viewer;
//! configured Vision analysis augments normalized image sources.

mod av;
mod image;
mod office;
mod pdf;
mod sheet;
mod text;
pub(crate) mod web;

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

/// What a source points at: a copied file, or a URL (weblink).
pub enum IngestInput {
    File(PathBuf),
    Url(String),
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
    pub page_count: Option<u32>,
    /// Set by the weblink parser from the page `<title>`.
    pub title_override: Option<String>,
    /// true if some non-fatal step (AI) was skipped — caller sets ready_partial.
    pub partial: bool,
    /// `Some("pending")` when the PDF has page(s) with no text layer that the
    /// Viewer should OCR (P12). `None` otherwise (text PDF, image, non-PDF).
    pub ocr_status: Option<String>,
}

struct Parsed {
    units: Vec<Unit>,
    page_count: Option<u32>,
    /// derived-relative image for `documents[0].image_rel` (images, PDF raster).
    first_image_rel: Option<String>,
    title_override: Option<String>,
    /// AI step skipped (image with no Vision yet) — ready_partial.
    partial: bool,
    /// 1-based PDF page numbers with no text layer (OCR candidates).
    scanned_pdf_pages: Vec<u32>,
}

pub fn derived_dir(project_dir: &Path, source_id: &str) -> PathBuf {
    project_dir.join("derived").join(source_id)
}

/// Run the whole pipeline for one source. Idempotent: wipes `derived/<sid>/`
/// and the source's documents/chunks first (docs/04 §0).
pub fn run(ctx: &IngestCtx, kind: SourceKind, input: &IngestInput) -> AppResult<Outcome> {
    let dd = derived_dir(ctx.project_dir, ctx.source_id);
    if dd.exists() {
        std::fs::remove_dir_all(&dd)?;
    }
    std::fs::create_dir_all(&dd)?;
    ctx.project_db.execute(
        "DELETE FROM documents WHERE source_id = ?1",
        [ctx.source_id],
    )?;
    ctx.app_db.execute(
        "DELETE FROM global_index WHERE source_id = ?1",
        [ctx.source_id],
    )?;

    let parsed = parse(ctx, kind, input, &dd)?;
    let Parsed {
        units,
        page_count,
        first_image_rel,
        title_override,
        partial,
        scanned_pdf_pages,
    } = parsed;

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

    for (di, u) in units.iter().enumerate() {
        let doc_id = Uuid::now_v7().to_string();
        let image_rel = if di == 0 {
            first_image_rel.as_deref()
        } else {
            None
        };
        ctx.project_db.execute(
            "INSERT INTO documents (id, source_id, ordinal, kind, title, text, image_rel, locator)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                doc_id,
                ctx.source_id,
                u.ordinal,
                u.kind,
                u.title,
                u.text,
                image_rel,
                u.locator.to_string()
            ],
        )?;

        for (i, piece) in chunk::split(&u.text).into_iter().enumerate() {
            let header = provenance_header(ctx.source_name, u);
            let body = format!("{header}\n{}", piece.text);
            let chunk_id = Uuid::now_v7().to_string();
            let locator =
                serde_json::json!({ "t": "line", "start": piece.start, "end": piece.end });
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
        page_count: page_count.or(Some(units.len() as u32)),
        title_override,
        partial,
        ocr_status: (!scanned_pdf_pages.is_empty()).then(|| "pending".to_string()),
    })
}

/// `data_dir` is `<...>/projects/<id>` → up two = the app data dir, where
/// whisper models live. Threading it through every ingest signature isn't worth
/// it for the one consumer.
fn data_dir_of(project_dir: &Path) -> &Path {
    project_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(project_dir)
}

/// Dispatch to the format parser. `dd` is `derived/<sid>/`.
fn parse(ctx: &IngestCtx, kind: SourceKind, input: &IngestInput, dd: &Path) -> AppResult<Parsed> {
    let file = |input: &IngestInput| -> AppResult<PathBuf> {
        match input {
            IngestInput::File(p) => Ok(p.clone()),
            IngestInput::Url(_) => Err(AppError::internal("expected a file, got a URL")),
        }
    };

    Ok(match kind {
        SourceKind::Text | SourceKind::Markdown | SourceKind::Code => {
            Parsed::plain(text::parse_text(kind, &file(input)?)?)
        }
        SourceKind::Json => Parsed::plain(text::parse_json(&file(input)?)?),
        SourceKind::Jsonl => Parsed::plain(text::parse_jsonl(&file(input)?)?),
        SourceKind::Pdf => {
            let parsed = pdf::parse_pdf(&file(input)?)?;
            let pc = parsed.units.len() as u32;
            Parsed {
                page_count: Some(pc),
                scanned_pdf_pages: parsed.scanned_pages,
                ..Parsed::plain(parsed.units)
            }
        }
        SourceKind::Sheet => {
            let p = file(input)?;
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let units = if ext == "xlsx" || ext == "xls" || ext == "xlsm" {
                office::parse_office(SourceKind::Sheet, &p)?
            } else {
                sheet::parse_sheet(&p)?
            };
            Parsed::plain(units)
        }
        SourceKind::Slides | SourceKind::Doc => {
            let units = office::parse_office(kind, &file(input)?)?;
            let pc = if kind == SourceKind::Slides {
                Some(units.len() as u32)
            } else {
                None
            };
            Parsed {
                page_count: pc,
                ..Parsed::plain(units)
            }
        }
        SourceKind::Image => {
            let ocr_on = crate::services::settings::get(ctx.app_db)
                .map(|s| s.ingest.ocr)
                .unwrap_or(true);
            let r = image::parse_image(&file(input)?, dd, data_dir_of(ctx.project_dir), ocr_on)?;
            Parsed {
                units: r.units,
                page_count: Some(1),
                first_image_rel: Some(r.derived_rel),
                title_override: None,
                partial: true, // Vision layout analysis still runs post-ingest
                scanned_pdf_pages: Vec::new(),
            }
        }
        SourceKind::Weblink => {
            let url = match input {
                IngestInput::Url(u) => u.clone(),
                IngestInput::File(_) => return Err(AppError::internal("weblink needs a URL")),
            };
            let r = web::fetch_and_parse(&url, dd, false)?;
            Parsed {
                units: r.units,
                page_count: None,
                first_image_rel: None,
                title_override: r.title,
                partial: false,
                scanned_pdf_pages: Vec::new(),
            }
        }
        SourceKind::Audio | SourceKind::Video => {
            let units = av::parse_av(ctx.app_db, data_dir_of(ctx.project_dir), &file(input)?)?;
            let n = units.len() as u32;
            Parsed {
                page_count: Some(n),
                ..Parsed::plain(units)
            }
        }
        SourceKind::Website => {
            let site_root = file(input)?;
            let rel: String = ctx.project_db.query_row(
                "SELECT rel_path FROM sources WHERE id = ?1",
                [ctx.source_id],
                |r| r.get(0),
            )?;
            let entry = rel
                .strip_prefix(&format!("sources/{}/", ctx.source_id))
                .unwrap_or("index.html")
                .to_string();
            let units = crate::services::website::parse_site(&site_root, &entry)?;
            let pc = units.len() as u32;
            Parsed {
                page_count: Some(pc),
                ..Parsed::plain(units)
            }
        }
    })
}

impl Parsed {
    fn plain(units: Vec<Unit>) -> Self {
        Parsed {
            units,
            page_count: None,
            first_image_rel: None,
            title_override: None,
            partial: false,
            scanned_pdf_pages: Vec::new(),
        }
    }
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
