//! File Modifier (docs/06 §6, FR-F1..F5). Two one-off tools that do not belong
//! to a project: images -> PDF, and pasted text -> organised Markdown / plain
//! TXT. No project sandbox here — the user picks the output path via the OS
//! dialog on the front-end.

use crate::domain::ai::{Role, StreamDelta, StreamDone, StreamError};
use crate::domain::file_modifier::*;
use crate::error::{AppError, AppResult};
use crate::services::ai::client::AiClient;
use crate::services::ai::{profiles, StreamRegistry};
use base64::Engine;
use printpdf::{
    Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, RawImage, RawImageData, RawImageFormat,
    XObjectTransform,
};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

mod prompts {
    pub const EN: &str = include_str!("ai/prompts/organizer.en.md");
    pub const JA: &str = include_str!("ai/prompts/organizer.ja.md");
    pub const ZH: &str = include_str!("ai/prompts/organizer.zh-Hans.md");
    pub fn organizer(lang: &str) -> &'static str {
        match lang {
            "ja" => JA,
            "zh-Hans" | "zh" => ZH,
            _ => EN,
        }
    }
}

// mm helpers
fn page_mm(size: &str, landscape: bool) -> (f32, f32) {
    let (w, h) = match size {
        "a3" => (297.0, 420.0),
        "letter" => (215.9, 279.4),
        _ => (210.0, 297.0), // a4
    };
    if landscape {
        (h, w)
    } else {
        (w, h)
    }
}
fn margin_mm(m: &str) -> f32 {
    match m {
        "sm" => 5.0,
        "md" => 10.0,
        "lg" => 20.0,
        _ => 0.0,
    }
}

pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// A small base64 data URL for previewing a picked image on the front-end
/// (the sandboxed `wakaru-asset://` scheme only serves project files).
pub fn image_preview(path: &str) -> AppResult<String> {
    let img = ::image::open(path)
        .map_err(|e| AppError::new("FM_BAD_IMAGE", "error.fm.badImage", e.to_string()))?;
    let thumb = img.resize(320, 320, ::image::imageops::FilterType::Triangle);
    let mut buf = Vec::new();
    thumb
        .write_to(
            &mut std::io::Cursor::new(&mut buf),
            ::image::ImageFormat::Jpeg,
        )
        .map_err(|e| AppError::internal(format!("thumb encode: {e}")))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

pub fn images_to_pdf(input: &ImagesToPdfInput) -> AppResult<WrittenFile> {
    if input.images.is_empty() {
        return Err(AppError::new(
            "FM_NO_IMAGES",
            "error.fm.noImages",
            "no images selected",
        ));
    }
    let mut doc = PdfDocument::new("WAKARU");
    let mut pages = Vec::with_capacity(input.images.len());

    for path in &input.images {
        let dyn_img = ::image::open(path).map_err(|e| {
            AppError::new("FM_BAD_IMAGE", "error.fm.badImage", format!("{path}: {e}"))
        })?;
        let (iw, ih) = (dyn_img.width() as f32, dyn_img.height() as f32);
        let img_landscape = iw > ih;

        let landscape = match input.orientation.as_str() {
            "portrait" => false,
            "landscape" => true,
            _ => img_landscape, // auto
        };

        let (pw, ph) = if input.page_size == "fit" {
            // "fit to image" — 96 dpi -> mm
            (px_to_mm(iw), px_to_mm(ih))
        } else {
            page_mm(&input.page_size, landscape)
        };

        let m = margin_mm(&input.margin);
        let (avail_w, avail_h) = (pw - 2.0 * m, ph - 2.0 * m);
        let img_ratio = iw / ih;
        let box_ratio = avail_w / avail_h;

        let (draw_w, draw_h) = if input.fit == "cover" {
            if img_ratio > box_ratio {
                (avail_h * img_ratio, avail_h)
            } else {
                (avail_w, avail_w / img_ratio)
            }
        } else if img_ratio > box_ratio {
            (avail_w, avail_w / img_ratio)
        } else {
            (avail_h * img_ratio, avail_h)
        };
        let x = m + (avail_w - draw_w) / 2.0;
        let y = m + (avail_h - draw_h) / 2.0;

        let rgb = dyn_img.to_rgb8();
        let (rgb_width, rgb_height) = (rgb.width() as usize, rgb.height() as usize);
        let image = RawImage {
            pixels: RawImageData::U8(rgb.into_raw()),
            width: rgb_width,
            height: rgb_height,
            data_format: RawImageFormat::RGB8,
            tag: Vec::new(),
        };
        let image_id = doc.add_image(&image);
        let scale_x = draw_w / px_to_mm(iw);
        let scale_y = draw_h / px_to_mm(ih);
        pages.push(PdfPage::new(
            Mm(pw),
            Mm(ph),
            vec![Op::UseXobject {
                id: image_id,
                transform: XObjectTransform {
                    translate_x: Some(Mm(x).into_pt()),
                    translate_y: Some(Mm(y).into_pt()),
                    scale_x: Some(scale_x),
                    scale_y: Some(scale_y),
                    dpi: Some(96.0),
                    ..Default::default()
                },
            }],
        ));
    }

    if let Some(parent) = Path::new(&input.dest_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut warnings = Vec::new();
    let bytes = doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    std::fs::write(&input.dest_path, bytes)?;
    Ok(WrittenFile {
        path: input.dest_path.clone(),
    })
}

fn px_to_mm(px: f32) -> f32 {
    px * 25.4 / 96.0
}

pub fn save_text(input: &SaveTextInput) -> AppResult<WrittenFile> {
    let mut path = input.dest_path.clone();
    let want_ext = if input.format == "md" { "md" } else { "txt" };
    if !path.to_lowercase().ends_with(&format!(".{want_ext}")) {
        path.push('.');
        path.push_str(want_ext);
    }
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &input.content)?;
    Ok(WrittenFile { path })
}

/// Stream an organised-Markdown conversion of `text` through the organizer role
/// (falls back to `chat`). Emits `stream://delta|done|error`.
pub async fn text_to_markdown(
    app: &AppHandle,
    reg: Arc<StreamRegistry>,
    app_db_path: &Path,
    text: String,
    ui_lang: String,
) -> AppResult<String> {
    let resolved = {
        let app_db = crate::storage::open(app_db_path)?;
        profiles::resolve(&app_db, Role::Organizer)?
    }
    .ok_or_else(|| {
        AppError::new(
            "AI_NOT_CONFIGURED",
            "error.ai.notConfigured",
            "no organizer/chat model",
        )
    })?;

    let system = prompts::organizer(&ui_lang).replace("{{text}}", &text);
    let model = resolved.model.clone();
    let (stream_id, token) = reg.start();
    let app = app.clone();
    let sid = stream_id.clone();

    tauri::async_runtime::spawn(async move {
        let client = match AiClient::new(
            resolved.protocol,
            &resolved.base_url,
            resolved.api_key,
            resolved.extra_headers,
            resolved.timeout_ms,
        ) {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "stream://error",
                    StreamError {
                        stream_id: sid.clone(),
                        error: e,
                    },
                );
                reg.finish(&sid);
                return;
            }
        };
        let messages = serde_json::json!([
            { "role": "system", "content": system },
            { "role": "user", "content": "Reformat now." }
        ]);
        let app2 = app.clone();
        let sid2 = sid.clone();
        let res = client
            .chat_stream(
                &model,
                messages,
                &resolved.params,
                &token,
                move |kind, t| {
                    let _ = app2.emit(
                        "stream://delta",
                        StreamDelta {
                            stream_id: sid2.clone(),
                            kind: kind.into(),
                            text: t.into(),
                        },
                    );
                },
            )
            .await;
        match res {
            Ok((usage, truncated, _)) => {
                let _ = app.emit(
                    "stream://done",
                    StreamDone {
                        stream_id: sid.clone(),
                        cancelled: token.is_cancelled(),
                        truncated,
                        usage,
                    },
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "stream://error",
                    StreamError {
                        stream_id: sid.clone(),
                        error: e,
                    },
                );
            }
        }
        reg.finish(&sid);
    });

    Ok(stream_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_to_pdf_writes_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let img = tmp.path().join("a.png");
        ::image::RgbImage::from_pixel(200, 300, ::image::Rgb([120, 130, 140]))
            .save(&img)
            .unwrap();
        let dest = tmp.path().join("out.pdf");
        let out = images_to_pdf(&ImagesToPdfInput {
            images: vec![img.to_string_lossy().into()],
            page_size: "a4".into(),
            orientation: "auto".into(),
            margin: "md".into(),
            fit: "contain".into(),
            dest_path: dest.to_string_lossy().into(),
        })
        .unwrap();
        let meta = std::fs::metadata(&out.path).unwrap();
        assert!(meta.len() > 200, "pdf looks empty: {} bytes", meta.len());
        // starts with the PDF magic
        let head = std::fs::read(&out.path).unwrap();
        assert_eq!(&head[..4], b"%PDF");
    }

    #[test]
    fn save_text_appends_the_right_extension_and_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let out = save_text(&SaveTextInput {
            content: "# hi\n\nbody".into(),
            dest_path: tmp.path().join("notes").to_string_lossy().into(),
            format: "md".into(),
        })
        .unwrap();
        assert!(out.path.ends_with("notes.md"));
        assert_eq!(std::fs::read_to_string(&out.path).unwrap(), "# hi\n\nbody");
    }
}
