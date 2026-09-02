//! Pure-Rust OCR (`ocrs` + `rten`; no ONNX, no C deps). The detection and
//! recognition models are downloaded once into `<data_dir>/models/ocr/` — the
//! same pattern as the local embedding model. OCR is best-effort: a missing or
//! failing model degrades to "no text found", never a panic.

use crate::error::{AppError, AppResult};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const DET_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const REC_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";
const DET_FILE: &str = "text-detection.rten";
const REC_FILE: &str = "text-recognition.rten";

/// Reject inputs above this so a pathological image can't exhaust memory.
const MAX_PIXELS: u64 = 30_000_000;

#[derive(Debug, Clone, Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct OcrWord {
    pub text: String,
    /// `[x, y, w, h]` as fractions of the page (0..1), resolution-independent.
    pub bbox: [f32; 4],
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct OcrLine {
    pub text: String,
    pub bbox: [f32; 4],
    pub words: Vec<OcrWord>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct OcrPage {
    pub lines: Vec<OcrLine>,
    pub width_px: u32,
    pub height_px: u32,
}

impl OcrPage {
    pub fn plain_text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn has_text(&self) -> bool {
        self.lines.iter().any(|l| !l.text.trim().is_empty())
    }

    /// Guard against OCR hallucinating "text" from noise/patterns in an image
    /// that has none: require a minimum length, a majority of alphanumeric
    /// characters (letters/digits/CJK), and some character variety.
    pub fn looks_like_text(&self) -> bool {
        let joined: String = self
            .lines
            .iter()
            .flat_map(|l| l.text.trim().chars())
            .collect();
        let total = joined.chars().count();
        if total < 6 {
            return false;
        }
        let alnum = joined.chars().filter(|c| c.is_alphanumeric()).count();
        let distinct = joined
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        (alnum as f32 / total as f32) >= 0.55 && distinct >= 3
    }
}

static ENGINE: OnceLock<Mutex<OcrEngine>> = OnceLock::new();
/// OCR is CPU-heavy; one page at a time process-wide (as with transcription).
static OCR_LOCK: Mutex<()> = Mutex::new(());

fn model_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("ocr")
}

/// Blocking. Fetch a model file if absent. Writes to `<name>.part` then renames
/// so a killed download never leaves a half file that looks complete.
fn ensure_model(dir: &Path, name: &str, url: &str) -> AppResult<PathBuf> {
    let path = dir.join(name);
    if std::fs::metadata(&path)
        .map(|m| m.len() > 0)
        .unwrap_or(false)
    {
        return Ok(path);
    }
    std::fs::create_dir_all(dir)?;
    let client = reqwest::blocking::Client::builder()
        .no_proxy() // consistent with the AI client (D-18)
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| AppError::internal(format!("http client: {e}")))?;
    let bytes = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())
        .map_err(|e| {
            AppError::new(
                "OCR_MODEL_DOWNLOAD",
                "errors.ocr.unavailable",
                format!("could not download the OCR model: {e}"),
            )
            .retriable()
        })?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

fn engine(data_dir: &Path) -> AppResult<&'static Mutex<OcrEngine>> {
    if let Some(e) = ENGINE.get() {
        return Ok(e);
    }
    let dir = model_dir(data_dir);
    let det_path = ensure_model(&dir, DET_FILE, DET_URL)?;
    let rec_path = ensure_model(&dir, REC_FILE, REC_URL)?;
    let load = |p: &Path| -> AppResult<rten::Model> {
        rten::Model::load(std::fs::read(p)?).map_err(|e| {
            AppError::new(
                "OCR_MODEL_LOAD",
                "errors.ocr.unavailable",
                format!("could not load the OCR model: {e}"),
            )
        })
    };
    let params = OcrEngineParams {
        detection_model: Some(load(&det_path)?),
        recognition_model: Some(load(&rec_path)?),
        ..Default::default()
    };
    let eng = OcrEngine::new(params).map_err(|e| {
        AppError::new(
            "OCR_INIT",
            "errors.ocr.unavailable",
            format!("OCR engine init failed: {e}"),
        )
    })?;
    let _ = ENGINE.set(Mutex::new(eng));
    Ok(ENGINE.get().expect("engine just set"))
}

/// OCR encoded image bytes (PNG / JPEG / …). Blocking + CPU-heavy — call from
/// `spawn_blocking`. Returns `Ok` with an empty `lines` when the image has no
/// readable text; `Err` only on a hard model / init / decode failure.
pub fn ocr_image_bytes(data_dir: &Path, encoded: &[u8]) -> AppResult<OcrPage> {
    let img = image::load_from_memory(encoded)
        .map_err(|e| AppError::new("OCR_IMAGE", "errors.ocr.image", format!("bad image: {e}")))?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || (w as u64) * (h as u64) > MAX_PIXELS {
        return Err(AppError::new(
            "OCR_IMAGE_SIZE",
            "errors.ocr.image",
            "image is outside the supported OCR size range",
        ));
    }
    let rgb = img.to_rgb8();
    let source = ImageSource::from_bytes(rgb.as_raw(), (w, h))
        .map_err(|e| AppError::new("OCR_IMAGE", "errors.ocr.image", format!("{e:?}")))?;

    let engine = engine(data_dir)?;
    let _serialise = OCR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let engine = engine.lock().unwrap_or_else(|p| p.into_inner());

    let input = engine.prepare_input(source).map_err(run_err)?;
    let words = engine.detect_words(&input).map_err(run_err)?;
    let line_rects = engine.find_text_lines(&input, &words);
    let text_lines = engine
        .recognize_text(&input, &line_rects)
        .map_err(run_err)?;

    let (fw, fh) = (w as f32, h as f32);
    // `l,t,w,h` are pixel coords from an ocrs bounding rect; return page fractions.
    let frac = |l: f32, t: f32, ww: f32, hh: f32| {
        [
            (l / fw).clamp(0.0, 1.0),
            (t / fh).clamp(0.0, 1.0),
            (ww / fw).clamp(0.0, 1.0),
            (hh / fh).clamp(0.0, 1.0),
        ]
    };

    let mut lines = Vec::new();
    for line in text_lines.into_iter().flatten() {
        let text = line.to_string();
        if text.trim().is_empty() {
            continue;
        }
        let words = line
            .words()
            .filter_map(|word| {
                let t = word.to_string();
                if t.trim().is_empty() {
                    return None;
                }
                let r = word.bounding_rect().to_f32();
                Some(OcrWord {
                    text: t,
                    bbox: frac(r.left(), r.top(), r.width(), r.height()),
                })
            })
            .collect();
        let r = line.bounding_rect().to_f32();
        lines.push(OcrLine {
            text,
            bbox: frac(r.left(), r.top(), r.width(), r.height()),
            words,
        });
    }
    Ok(OcrPage {
        lines,
        width_px: w,
        height_px: h,
    })
}

/// ocrs returns `anyhow::Error`; keep only its message (secret-free — these are
/// tensor/decode errors, not network bodies).
fn run_err(e: impl std::fmt::Display) -> AppError {
    AppError::new("OCR_RUN", "errors.ocr.unavailable", e.to_string())
}

// ───────────────── scanned-PDF page OCR (Viewer-driven, P12/4) ─────────────────

use rusqlite::{params, Connection, OptionalExtension};

fn ocr_dir(project_dir: &Path, source_id: &str) -> PathBuf {
    crate::services::ingest::derived_dir(project_dir, source_id).join("ocr")
}

/// OCR one rasterised PDF page (PNG bytes) and fold the text into search:
/// replace the page's `documents.text`, rebuild that document's chunks and its
/// `global_index` row, and cache `ocr/pNNNN.{png,json}` for the sandwich PDF.
/// Best-effort — a page with no readable text is left as-is, not an error.
#[allow(clippy::too_many_arguments)]
pub fn apply_pdf_page(
    project_db: &Connection,
    app_db: &Connection,
    data_dir: &Path,
    project_dir: &Path,
    project_id: &str,
    source_id: &str,
    source_name: &str,
    page: u32,
    total: u32,
    png: &[u8],
) -> AppResult<OcrPage> {
    let dir = ocr_dir(project_dir, source_id);
    std::fs::create_dir_all(&dir)?;
    let _ = std::fs::write(dir.join(format!("p{page:04}.png")), png);

    let ocr_page = ocr_image_bytes(data_dir, png)?;
    let _ = std::fs::write(
        dir.join(format!("p{page:04}.json")),
        serde_json::to_vec(&ocr_page).unwrap_or_default(),
    );
    if !ocr_page.looks_like_text() {
        return Ok(ocr_page); // nothing usable — leave the placeholder text
    }

    let ocr_text = ocr_page.plain_text();
    let new_text = format!("[ページ {page} / {total}]\n{ocr_text}");

    let row: Option<(String, String)> = project_db
        .query_row(
            "SELECT id, text FROM documents WHERE source_id = ?1 AND ordinal = ?2",
            params![source_id, page],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((doc_id, existing)) = row else {
        return Ok(ocr_page);
    };
    // Only replace a page that has no real text layer (the ingest placeholder).
    // Never overwrite a page whose text came from the PDF itself.
    if !existing.contains("（テキスト層なし）") {
        return Ok(ocr_page);
    }

    project_db.execute(
        "UPDATE documents SET text = ?2 WHERE id = ?1",
        params![doc_id, new_text],
    )?;
    project_db.execute("DELETE FROM chunks WHERE document_id = ?1", [&doc_id])?;

    let header = format!("《{source_name} / page {page}》");
    let now = crate::storage::migrate::now_iso8601();
    for (i, piece) in crate::services::chunk::split(&new_text)
        .into_iter()
        .enumerate()
    {
        let body = format!("{header}\n{}", piece.text);
        project_db.execute(
            "INSERT INTO chunks
               (id, source_id, document_id, ordinal, text, text_bigram, tokens, locator, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                uuid::Uuid::now_v7().to_string(),
                source_id,
                doc_id,
                i as i64,
                body,
                crate::services::retrieval::cjk_bigram(&body),
                piece.tokens as i64,
                serde_json::json!({ "t": "page", "page": page }).to_string(),
                now
            ],
        )?;
    }

    // Refresh the cross-project mirror row for this page.
    let ref_id = format!("{source_id}#{page}");
    app_db.execute(
        "DELETE FROM global_index WHERE source_id = ?1 AND kind = 'chunk' AND ref_id = ?2",
        params![source_id, ref_id],
    )?;
    app_db.execute(
        "INSERT INTO global_index (project_id, source_id, kind, ref_id, title, body, body_raw)
         VALUES (?1, ?2, 'chunk', ?3, ?4, ?5, ?6)",
        params![
            project_id,
            source_id,
            ref_id,
            format!("p.{page}"),
            crate::services::retrieval::cjk_bigram(&new_text),
            new_text
        ],
    )?;

    Ok(ocr_page)
}

/// After every text-less page is OCR'd: set `ocr_status`, and (best-effort) build
/// `derived/<sid>/searchable.pdf` from the cached page rasters + boxes.
pub fn finalize_pdf(
    project_db: &Connection,
    project_dir: &Path,
    source_id: &str,
    all_ok: bool,
) -> AppResult<Option<String>> {
    let status = if all_ok { "done" } else { "partial" };
    project_db.execute(
        "UPDATE sources SET ocr_status = ?2 WHERE id = ?1",
        params![source_id, status],
    )?;

    let dir = ocr_dir(project_dir, source_id);
    let mut nums: Vec<u32> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_prefix('p')?
                .strip_suffix(".png")?
                .parse()
                .ok()
        })
        .collect();
    nums.sort_unstable();
    if nums.is_empty() {
        return Ok(None);
    }

    let mut pages = Vec::new();
    for n in nums {
        let png = match std::fs::read(dir.join(format!("p{n:04}.png"))) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let (w, h) = image::load_from_memory(&png)
            .map(|i| (i.width() as f32 * 0.75, i.height() as f32 * 0.75)) // ~96dpi px -> pt
            .unwrap_or((595.0, 842.0));
        let lines: Vec<(String, [f32; 4])> = std::fs::read(dir.join(format!("p{n:04}.json")))
            .ok()
            .and_then(|b| serde_json::from_slice::<OcrPage>(&b).ok())
            .map(|p| p.lines.into_iter().map(|l| (l.text, l.bbox)).collect())
            .unwrap_or_default();
        pages.push(crate::services::pdf_text::SandwichPage {
            image: png,
            width_pt: w,
            height_pt: h,
            lines,
        });
    }

    let built = match crate::services::pdf_text::render_sandwich(&pages) {
        Ok(bytes) => {
            let out =
                crate::services::ingest::derived_dir(project_dir, source_id).join("searchable.pdf");
            std::fs::write(&out, bytes)?;
            Some(format!("derived/{source_id}/searchable.pdf"))
        }
        Err(_) => None, // no CJK font etc. — the Viewer overlay still works
    };

    // The page rasters were only needed for the sandwich; keep the small `.json`
    // box files for the Viewer text overlay.
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        if entry.path().extension().is_some_and(|e| e == "png") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(built)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(lines: &[(&str, [f32; 4])]) -> OcrPage {
        OcrPage {
            lines: lines
                .iter()
                .map(|(t, b)| OcrLine {
                    text: (*t).into(),
                    bbox: *b,
                    words: vec![],
                })
                .collect(),
            width_px: 1000,
            height_px: 1400,
        }
    }

    #[test]
    fn looks_like_text_rejects_noise_and_accepts_real_text() {
        let real = page(&[
            ("The quarterly report is attached.", [0.1, 0.1, 0.6, 0.02]),
            ("見積書 2026年 合計 12,300円", [0.1, 0.2, 0.5, 0.02]),
        ]);
        assert!(real.looks_like_text());

        for junk in [
            "//..--//", // punctuation soup
            "a a a",    // too short + no variety
            "|| || ||", // symbols
            "x",        // trivial
        ] {
            assert!(
                !page(&[(junk, [0.0, 0.0, 0.1, 0.1])]).looks_like_text(),
                "should reject {junk:?}"
            );
        }
    }

    #[test]
    fn plain_text_joins_lines_and_has_text_detects_content() {
        let p = page(&[
            ("first", [0.1, 0.1, 0.3, 0.02]),
            ("second", [0.1, 0.2, 0.4, 0.02]),
        ]);
        assert_eq!(p.plain_text(), "first\nsecond");
        assert!(p.has_text());

        let blank = page(&[("   ", [0.0, 0.0, 0.0, 0.0])]);
        assert!(!blank.has_text());
        assert!(OcrPage {
            lines: vec![],
            width_px: 1,
            height_px: 1
        }
        .plain_text()
        .is_empty());
    }

    #[test]
    fn oversized_or_empty_images_are_rejected_without_running_the_engine() {
        let dir = std::env::temp_dir();
        // 1x1 white PNG.
        let png = image::RgbImage::from_pixel(1, 1, image::Rgb([255, 255, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(png)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        // A 1x1 image has text-free content; this must not error and must not
        // require the models (engine() is only reached after the size gate).
        let bytes = buf.into_inner();
        // Truncate to make it an invalid image → deterministic bad-image error.
        let err = ocr_image_bytes(&dir, &bytes[..4]).unwrap_err();
        assert_eq!(err.code, "OCR_IMAGE");
    }
}
