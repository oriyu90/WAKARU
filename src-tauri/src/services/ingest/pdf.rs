//! PDF text extraction (docs/04 §4), one `documents` row per page. The Viewer
//! renders the original file with PDF.js; this module builds searchable text.

use super::Unit;
use crate::error::{AppError, AppResult};
use std::path::Path;

pub struct PdfParse {
    pub units: Vec<Unit>,
    /// 1-based page numbers with no extractable text layer — candidates for OCR.
    pub scanned_pages: Vec<u32>,
}

pub fn parse_pdf(path: &Path) -> AppResult<PdfParse> {
    let bytes = std::fs::read(path)?;
    let pages = pdf_extract::extract_text_from_mem_by_pages(&bytes).map_err(|e| {
        AppError::new(
            "SOURCE_PARSE",
            "error.source.parse",
            format!("could not read PDF: {e}"),
        )
    })?;

    let total = pages.len();
    let mut units = Vec::new();
    let mut scanned_pages = Vec::new();
    for (i, raw) in pages.into_iter().enumerate() {
        let page = (i + 1) as u32;
        let text = normalize(&raw);
        if text.trim().is_empty() {
            // Scanned page with no text layer. Keep a placeholder so the page
            // remains a stable citation unit; the Viewer will OCR it (P12).
            scanned_pages.push(page);
            units.push(Unit {
                ordinal: page,
                kind: "page",
                title: Some(format!("p.{page}")),
                text: format!("[ページ {page} / {total}] （テキスト層なし）"),
                locator: serde_json::json!({ "t": "page", "page": page }),
            });
            continue;
        }
        units.push(Unit {
            ordinal: page,
            kind: "page",
            title: Some(format!("p.{page}")),
            text: format!("[ページ {page} / {total}]\n{text}"),
            locator: serde_json::json!({ "t": "page", "page": page }),
        });
    }

    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "PDF has no pages",
        ));
    }
    Ok(PdfParse {
        units,
        scanned_pages,
    })
}

/// pdf-extract inserts hard line breaks mid-sentence; collapse single newlines
/// inside a paragraph, keep blank lines.
fn normalize(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_blank = true;
    for line in raw.lines() {
        let l = line.trim_end();
        if l.is_empty() {
            if !prev_blank {
                out.push_str("\n\n");
            }
            prev_blank = true;
        } else {
            if !prev_blank && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push_str(l.trim());
            prev_blank = false;
        }
    }
    out.trim().to_string()
}
