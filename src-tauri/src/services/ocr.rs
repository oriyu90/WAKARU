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

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct OcrWord {
    pub text: String,
    /// `[x, y, w, h]` as fractions of the page (0..1), resolution-independent.
    pub bbox: [f32; 4],
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct OcrLine {
    pub text: String,
    pub bbox: [f32; 4],
    pub words: Vec<OcrWord>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
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
