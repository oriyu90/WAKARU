//! Image normalisation (docs/04 §1): EXIF-rotate, cap the long side at 2048 px,
//! write a normalised copy to `derived/<sid>/pages/0001.png`. When the image
//! carries text, OCR (`services::ocr`) adds a searchable `ocr` unit plus
//! `derived/<sid>/ocr.txt` and `derived/<sid>/ocr.json` sidecars. A configured
//! Vision role adds layout/diagram analysis separately.

use super::Unit;
use crate::error::{AppError, AppResult};
use crate::services::ocr;
use image::imageops::FilterType;
use std::io::BufReader;
use std::path::Path;

const MAX_SIDE: u32 = 2048;

pub struct ImageResult {
    pub units: Vec<Unit>,
    /// Written under `derived/<sid>/` — relative path.
    pub derived_rel: String,
}

pub fn parse_image(
    abs_path: &Path,
    derived_dir: &Path,
    data_dir: &Path,
    ocr_enabled: bool,
) -> AppResult<ImageResult> {
    let img = image::open(abs_path).map_err(|e| {
        AppError::new(
            "SOURCE_PARSE",
            "error.source.parse",
            format!("bad image: {e}"),
        )
    })?;

    let orientation = read_exif_orientation(abs_path).unwrap_or(1);
    let mut img = apply_orientation(img, orientation);

    if img.width().max(img.height()) > MAX_SIDE {
        img = img.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3);
    }

    let pages = derived_dir.join("pages");
    std::fs::create_dir_all(&pages)?;
    let out = pages.join("0001.png");
    img.to_rgba8()
        .save(&out)
        .map_err(|e| AppError::internal(format!("write normalised image: {e}")))?;

    let name = abs_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image")
        .to_string();

    let mut units = vec![Unit {
        ordinal: 1,
        kind: "image",
        title: Some(name.clone()),
        text: format!("[画像] {name}\n（{}×{}）", img.width(), img.height()),
        locator: serde_json::json!({ "t": "region", "bbox": [0.0, 0.0, 1.0, 1.0] }),
    }];

    // Best-effort OCR of the normalised copy. Failure (offline, model download,
    // decode) is non-fatal — the image is still viewable, just not text-searchable.
    if ocr_enabled {
        if let Some(unit) = run_ocr(&out, derived_dir, data_dir) {
            units.push(unit);
        }
    }

    Ok(ImageResult {
        units,
        derived_rel: "pages/0001.png".into(),
    })
}

/// OCR `png_path`; on success with readable text, write the `ocr.txt` / `ocr.json`
/// sidecars and return a searchable `ocr` unit. Returns `None` on no-text or any
/// error.
fn run_ocr(png_path: &Path, derived_dir: &Path, data_dir: &Path) -> Option<Unit> {
    let bytes = std::fs::read(png_path).ok()?;
    let page = match ocr::ocr_image_bytes(data_dir, &bytes) {
        Ok(p) if p.looks_like_text() => p,
        _ => return None,
    };
    let text = page.plain_text();
    let _ = std::fs::write(derived_dir.join("ocr.txt"), &text);
    if let Ok(json) = serde_json::to_vec(&page) {
        let _ = std::fs::write(derived_dir.join("ocr.json"), json);
    }
    Some(Unit {
        ordinal: 2,
        kind: "ocr",
        title: Some("OCR".into()),
        text,
        locator: serde_json::json!({ "t": "region", "bbox": [0.0, 0.0, 1.0, 1.0] }),
    })
}

fn read_exif_orientation(path: &Path) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
    field.value.get_uint(0)
}

fn apply_orientation(img: image::DynamicImage, o: u32) -> image::DynamicImage {
    use image::DynamicImage as D;
    match o {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => D::ImageRgba8(img.rotate90().fliph().to_rgba8()),
        6 => img.rotate90(),
        7 => D::ImageRgba8(img.rotate270().fliph().to_rgba8()),
        8 => img.rotate270(),
        _ => img,
    }
}
