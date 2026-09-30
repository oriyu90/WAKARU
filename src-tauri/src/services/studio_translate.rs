//! v1.5.0 C: full-source translation to PDF as an independent module.
//!
//! The Studio tool `translate_source_document(sourceId, targetLanguage,
//! outputPath)` translates **all** extracted pages of one source and renders a
//! real PDF. Pure helpers here are unit-tested without a model; the async
//! per-page loop lives in `services::studio` so it can reuse the tab's
//! `AiClient`, approval result and cancellation token.
//!
//! Safety posture (unchanged from `build_document`):
//! - No new persistent tables. In-flight translations stay in bounded memory.
//! - No artifact row is written before every page has a non-empty translation
//!   and the PDF bytes verify as `%PDF-`.
//! - Original images / layout are not preserved — the PDF re-flows extracted
//!   text with one headed section per source page.

use crate::error::{AppError, AppResult};
use crate::services::doc_builder::DocSection;
use rusqlite::OptionalExtension;

/// Hard caps for one translation job. Over-limit requests fail with guidance
/// instead of growing memory or emitting a truncated PDF.
pub const MAX_TRANSLATE_PAGES: usize = 50;
pub const MAX_TRANSLATE_INPUT_CHARS: usize = 200_000;
pub const MAX_TRANSLATE_OUTPUT_BYTES: u64 = 20 * 1024 * 1024;
pub const MIN_PDF_BYTES: usize = 800;
/// One model request carries at most this many characters. Longer pages are
/// split before translation and retried once with a smaller split.
pub const MAX_CHUNK_CHARS: usize = 8_000;
pub const RETRY_CHUNK_CHARS: usize = 4_000;

fn bad(code: &str, key: &str, msg: impl Into<String>) -> AppError {
    AppError::new(code, key, msg)
}

/// Normalise a UI language / tool argument to `ja` | `en` | `zh-Hans`.
pub fn normalize_target_language(raw: &str) -> Option<String> {
    match raw.trim() {
        "ja" | "jp" | "japanese" | "日本語" => Some("ja".into()),
        "en" | "english" | "英語" => Some("en".into()),
        "zh-Hans" | "zh" | "cn" | "简体中文" | "簡体字" | "中国語(簡体字)" => {
            Some("zh-Hans".into())
        }
        _ => None,
    }
}

pub fn target_language_label(code: &str) -> &'static str {
    match code {
        "ja" => "日本語",
        "zh-Hans" | "zh" => "简体中文",
        _ => "English",
    }
}

/// One extracted source page in `ordinal` order.
#[derive(Debug, Clone, PartialEq)]
pub struct SourcePage {
    pub ordinal: i64,
    pub title: Option<String>,
    pub text: String,
}

/// Read every extracted page of `source_id` in ordinal order.
pub fn collect_pages(
    db: &rusqlite::Connection,
    source_id: &str,
) -> AppResult<(String, Vec<SourcePage>)> {
    let source_name: String = db
        .query_row(
            "SELECT original_name FROM sources WHERE id = ?1",
            [source_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(AppError::from)?
        .ok_or_else(|| {
            bad(
                "STUDIO_TRANSLATE_NO_SOURCE",
                "error.studio.translateNoSource",
                format!("unknown source: {source_id}"),
            )
        })?;
    let mut stmt = db
        .prepare("SELECT ordinal, title, text FROM documents WHERE source_id = ?1 ORDER BY ordinal")
        .map_err(AppError::from)?;
    let pages: Vec<SourcePage> = stmt
        .query_map([source_id], |r| {
            Ok(SourcePage {
                ordinal: r.get(0)?,
                title: r.get(1)?,
                text: r.get(2)?,
            })
        })
        .map_err(AppError::from)?
        .collect::<rusqlite::Result<_>>()
        .map_err(AppError::from)?;
    if pages.is_empty() {
        return Err(bad(
            "STUDIO_TRANSLATE_EMPTY",
            "error.studio.translateEmpty",
            "the source has no extracted pages",
        ));
    }
    Ok((source_name, pages))
}

/// Pages whose extracted text is empty after trimming. These are reported, not
/// silently skipped.
pub fn empty_pages(pages: &[SourcePage]) -> Vec<i64> {
    pages
        .iter()
        .filter(|p| p.text.trim().is_empty())
        .map(|p| p.ordinal)
        .collect()
}

/// Validate page count / input volume / output path before any model call.
pub fn validate_plan(
    pages: &[SourcePage],
    target_language: &str,
    output_path: &str,
) -> AppResult<()> {
    if normalize_target_language(target_language).is_none() {
        return Err(bad(
            "STUDIO_TRANSLATE_ARG",
            "error.studio.translateArg",
            "targetLanguage must be ja, en or zh-Hans",
        ));
    }
    if pages.len() > MAX_TRANSLATE_PAGES {
        return Err(bad(
            "STUDIO_TRANSLATE_TOO_LARGE",
            "error.studio.translateTooLarge",
            format!(
                "this source has {} pages (limit {MAX_TRANSLATE_PAGES}); split it or pick a page range",
                pages.len()
            ),
        ));
    }
    let total: usize = pages.iter().map(|p| p.text.chars().count()).sum();
    if total > MAX_TRANSLATE_INPUT_CHARS {
        return Err(bad(
            "STUDIO_TRANSLATE_TOO_LARGE",
            "error.studio.translateTooLarge",
            format!(
                "input is {total} characters (limit {MAX_TRANSLATE_INPUT_CHARS}); split the source first"
            ),
        ));
    }
    if total == 0 {
        return Err(bad(
            "STUDIO_TRANSLATE_EMPTY",
            "error.studio.translateEmpty",
            "the source has no translatable text",
        ));
    }
    let trimmed = output_path.trim();
    if trimmed.is_empty() {
        return Err(bad(
            "STUDIO_TRANSLATE_ARG",
            "error.studio.translateArg",
            "missing argument: outputPath",
        ));
    }
    if !trimmed.to_ascii_lowercase().ends_with(".pdf") {
        return Err(bad(
            "STUDIO_TRANSLATE_ARG",
            "error.studio.translateArg",
            "outputPath must end with .pdf",
        ));
    }
    Ok(())
}

/// Split one page into bounded chunks, preferring paragraph boundaries.
pub fn split_for_translate(text: &str, max_chars: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut cur = String::new();
    let mut cur_len = 0usize;
    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        let len = para.chars().count() + 2;
        if cur_len + len > max_chars && !cur.is_empty() {
            chunks.push(std::mem::take(&mut cur));
            cur_len = 0;
        }
        // A single over-long paragraph falls back to a hard char split.
        if len > max_chars {
            if !cur.is_empty() {
                chunks.push(std::mem::take(&mut cur));
                cur_len = 0;
            }
            let chars: Vec<char> = para.chars().collect();
            for win in chars.chunks(max_chars) {
                chunks.push(win.iter().collect());
            }
            continue;
        }
        if !cur.is_empty() {
            cur.push_str("\n\n");
        }
        cur.push_str(para);
        cur_len += len;
    }
    if !cur.trim().is_empty() {
        chunks.push(cur);
    }
    chunks
        .into_iter()
        .filter(|c| !c.trim().is_empty())
        .collect()
}

/// Build one headed section per source page, keeping page correspondence.
pub fn sections_from_translations(
    source_name: &str,
    target_language: &str,
    pages: &[SourcePage],
    translated: &[String],
) -> Vec<DocSection> {
    let label = target_language_label(target_language);
    pages
        .iter()
        .zip(translated.iter())
        .map(|(page, body)| {
            let title_part = page
                .title
                .as_deref()
                .filter(|t| !t.trim().is_empty())
                .map(|t| format!(" — {}", t.trim()))
                .unwrap_or_default();
            DocSection {
                level: 1,
                heading: Some(format!(
                    "p.{} {source_name}{title_part} ({label})",
                    page.ordinal
                )),
                body: body.clone(),
            }
        })
        .collect()
}

/// Verify rendered PDF bytes before any artifact row is written.
pub fn verify_pdf_bytes(bytes: &[u8]) -> AppResult<()> {
    if bytes.len() < 5 || &bytes[..5] != b"%PDF-" {
        return Err(bad(
            "STUDIO_TRANSLATE_RENDER",
            "error.studio.translateRender",
            "the renderer did not produce a PDF",
        ));
    }
    if bytes.len() < MIN_PDF_BYTES {
        return Err(bad(
            "STUDIO_TRANSLATE_RENDER",
            "error.studio.translateRender",
            "the rendered PDF is suspiciously small",
        ));
    }
    if bytes.len() as u64 > MAX_TRANSLATE_OUTPUT_BYTES {
        return Err(bad(
            "STUDIO_TRANSLATE_TOO_LARGE",
            "error.studio.translateTooLarge",
            "the rendered PDF exceeds the size limit",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(n: i64, text: &str) -> SourcePage {
        SourcePage {
            ordinal: n,
            title: Some(format!("Page {n}")),
            text: text.into(),
        }
    }

    #[test]
    fn normalises_the_three_ui_languages() {
        assert_eq!(normalize_target_language("ja").as_deref(), Some("ja"));
        assert_eq!(normalize_target_language("日本語").as_deref(), Some("ja"));
        assert_eq!(normalize_target_language("en").as_deref(), Some("en"));
        assert_eq!(
            normalize_target_language("zh-Hans").as_deref(),
            Some("zh-Hans")
        );
        assert!(normalize_target_language("fr").is_none());
    }

    #[test]
    fn detects_empty_pages_instead_of_skipping() {
        let pages = vec![page(1, "hello"), page(2, "   \n "), page(3, "world")];
        assert_eq!(empty_pages(&pages), vec![2]);
    }

    #[test]
    fn rejects_over_limit_plans_before_any_model_call() {
        let pages: Vec<SourcePage> = (1..=60).map(|n| page(n, "x")).collect();
        assert!(validate_plan(&pages, "ja", "out.pdf").is_err());
        assert!(validate_plan(&[page(1, "x")], "fr", "out.pdf").is_err());
        assert!(validate_plan(&[page(1, "x")], "ja", "out.md").is_err());
        assert!(validate_plan(&[page(1, "x")], "ja", "out.pdf").is_ok());
    }

    #[test]
    fn splits_long_pages_on_paragraphs_with_hard_fallback() {
        let text = (0..30)
            .map(|i| format!("para {i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = split_for_translate(&text, 40);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.chars().count() <= 80));
        assert_eq!(
            split_for_translate("short", 8000),
            vec!["short".to_string()]
        );
    }

    #[test]
    fn sections_keep_page_order_and_correspondence() {
        let pages = vec![page(2, "b"), page(1, "a")];
        let out = sections_from_translations("src.pdf", "ja", &pages, &["TB".into(), "TA".into()]);
        assert_eq!(out.len(), 2);
        assert!(out[0].heading.as_deref().unwrap().contains("p.2"));
        assert!(out[1].heading.as_deref().unwrap().contains("p.1"));
        assert!(out[0].heading.as_deref().unwrap().contains("日本語"));
    }

    #[test]
    fn pdf_verification_rejects_non_pdf_and_tiny_output() {
        assert!(verify_pdf_bytes(b"not a pdf at all.............").is_err());
        assert!(verify_pdf_bytes(b"%PDF-tiny").is_err());
        let mut good = b"%PDF-1.7\n".to_vec();
        good.extend(vec![b'x'; 2000]);
        assert!(verify_pdf_bytes(&good).is_ok());
    }

    #[test]
    fn nine_page_fixture_translates_in_order_with_mock() {
        // Mirrors the v1.5.0 failure case shape: 9 pages of extracted text.
        let pages: Vec<SourcePage> = (1..=9)
            .map(|n| page(n, &format!("source sentence {n}")))
            .collect();
        validate_plan(&pages, "ja", "translated.pdf").unwrap();
        // Mock translator: deterministic per-page Japanese output.
        let translated: Vec<String> = pages
            .iter()
            .map(|p| format!("翻訳文 p.{}: {}", p.ordinal, p.text))
            .collect();
        assert_eq!(translated.len(), 9);
        let sections = sections_from_translations("fixture.pdf", "ja", &pages, &translated);
        assert_eq!(sections.len(), 9);
        for (i, s) in sections.iter().enumerate() {
            assert!(s
                .heading
                .as_deref()
                .unwrap()
                .contains(&format!("p.{}", i + 1)));
            assert!(s.body.contains("翻訳文"));
        }
        let req = crate::services::doc_builder::DocRequest {
            title: "fixture 翻訳",
            toc: false,
            sections: &sections,
        };
        match crate::services::doc_builder::render("pdf", &req) {
            Ok(bytes) => {
                assert_eq!(&bytes[..5], b"%PDF-");
                verify_pdf_bytes(&bytes).unwrap();
            }
            Err(e) => assert_eq!(e.code, "DOC_BUILD_PDF_UNAVAILABLE"),
        }
    }
}
