//! PDF text extraction (docs/04 §4). Phase 1: text only, one `documents` row per
//! page. Rasterisation to page images is deferred to Phase 2 (D-09) when the
//! Viewer needs them.

use super::Unit;
use crate::error::{AppError, AppResult};
use std::path::Path;

pub fn parse_pdf(path: &Path) -> AppResult<Vec<Unit>> {
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
    for (i, raw) in pages.into_iter().enumerate() {
        let text = normalize(&raw);
        if text.trim().is_empty() {
            // Scanned page with no text layer — OCR (Vision / Tesseract) is
            // Phase 4. Keep a placeholder so the page still has a citation unit.
            units.push(Unit {
                ordinal: (i + 1) as u32,
                kind: "page",
                title: Some(format!("p.{}", i + 1)),
                text: format!("[ページ {} / {}] （テキスト層なし。解析は後で補完されます）", i + 1, total),
                locator: serde_json::json!({ "t": "page", "page": i + 1 }),
            });
            continue;
        }
        units.push(Unit {
            ordinal: (i + 1) as u32,
            kind: "page",
            title: Some(format!("p.{}", i + 1)),
            text: format!("[ページ {} / {}]\n{}", i + 1, total, text),
            locator: serde_json::json!({ "t": "page", "page": i + 1 }),
        });
    }

    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "PDF has no pages",
        ));
    }
    Ok(units)
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
