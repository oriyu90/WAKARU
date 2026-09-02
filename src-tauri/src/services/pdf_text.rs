//! PDF text output for `build_document` and the OCR "sandwich" PDF (P12).
//!
//! No font is bundled: a CJK-capable sans face is taken from the operating
//! system via `fontdb` and subset-embedded by `printpdf`. macOS (the distributed
//! platform) always has one; if none is found, callers fall back (a
//! `build_document` `pdf` request writes the `.md` version instead).

use crate::error::AppError;
use crate::services::doc_builder::{parse_blocks, Block, DocRequest, Span};
use printpdf::*;
use std::sync::OnceLock;

pub const AVAILABILITY_NOTE: &str =
    "no CJK-capable system font was found, so a Markdown version was saved instead — export it to PDF from your editor.";

// A4, 20mm margins.
const PAGE_W_MM: f32 = 210.0;
const PAGE_H_MM: f32 = 297.0;
const MARGIN_MM: f32 = 20.0;
const CONTENT_W_MM: f32 = PAGE_W_MM - 2.0 * MARGIN_MM;

const SZ_TITLE: f32 = 20.0;
const SZ_H: [f32; 4] = [15.0, 13.0, 12.0, 11.0];
const SZ_BODY: f32 = 10.5;
const SZ_CODE: f32 = 9.5;
const LEADING: f32 = 1.45;

fn err(code: &str, msg: impl Into<String>) -> AppError {
    AppError::new(code, "errors.doc.build", msg)
}

// ───────────────────── system font ─────────────────────

static FONT_BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// Bytes of a system sans face that covers CJK ideographs and kana, cached.
fn cjk_font_bytes() -> Option<&'static [u8]> {
    FONT_BYTES.get_or_init(load_cjk_font).as_deref()
}

fn load_cjk_font() -> Option<Vec<u8>> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    // Preferred families, best first, across macOS / Windows / common Linux.
    const PREFERRED: &[&str] = &[
        "Hiragino Sans",
        "Hiragino Kaku Gothic ProN",
        "YuGothic",
        "Yu Gothic",
        "PingFang SC",
        "Noto Sans CJK JP",
        "Noto Sans CJK SC",
        "Noto Sans JP",
        "Noto Sans SC",
        "Source Han Sans",
        "Microsoft YaHei",
        "SimSun",
        "WenQuanYi Zen Hei",
    ];
    // Probe glyphs: 漢 (CJK ideograph), あ (hiragana).
    let covers_cjk = |id: fontdb::ID, db: &fontdb::Database| -> bool {
        db.with_face_data(id, |data, index| {
            ttf_parser::Face::parse(data, index)
                .map(|f| f.glyph_index('\u{6f22}').is_some() && f.glyph_index('\u{3042}').is_some())
                .unwrap_or(false)
        })
        .unwrap_or(false)
    };
    let bytes_of = |id: fontdb::ID, db: &fontdb::Database| -> Option<Vec<u8>> {
        db.with_face_data(id, |data, _| data.to_vec())
    };

    for name in PREFERRED {
        let q = fontdb::Query {
            families: &[fontdb::Family::Name(name)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        if let Some(id) = db.query(&q) {
            if covers_cjk(id, &db) {
                if let Some(b) = bytes_of(id, &db) {
                    return Some(b);
                }
            }
        }
    }
    // Fallback: scan every face for CJK coverage.
    let ids: Vec<fontdb::ID> = db.faces().map(|f| f.id).collect();
    for id in ids {
        if covers_cjk(id, &db) {
            if let Some(b) = bytes_of(id, &db) {
                return Some(b);
            }
        }
    }
    None
}

/// A parsed face + its `printpdf` document id — one per document.
struct Face {
    parsed: ParsedFont,
}

impl Face {
    fn load() -> Result<Face, AppError> {
        let bytes =
            cjk_font_bytes().ok_or_else(|| err("DOC_BUILD_PDF_UNAVAILABLE", AVAILABILITY_NOTE))?;
        let mut warn = Vec::new();
        let parsed = ParsedFont::from_bytes(bytes, 0, &mut warn)
            .ok_or_else(|| err("DOC_BUILD_PDF_FONT", "the system font could not be parsed"))?;
        Ok(Face { parsed })
    }
}

/// Rough advance width in mm for `text` at `size_pt`. CJK / full-width glyphs are
/// ~1em, everything else ~0.55em. Good enough for paragraph wrapping.
fn text_width_mm(text: &str, size_pt: f32) -> f32 {
    let em_mm = size_pt * 0.352_777_8; // pt -> mm
    text.chars()
        .map(|c| if is_wide(c) { em_mm } else { em_mm * 0.55 })
        .sum()
}

fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF |
        0x4E00..=0x9FFF | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF |
        0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x20000..=0x3FFFD)
}

/// Break `text` into lines no wider than `max_mm` at `size_pt`. Breaks on spaces
/// where possible; falls back to per-character breaks (CJK, long tokens).
fn wrap(text: &str, size_pt: f32, max_mm: f32) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in split_keep_spaces(text) {
        let candidate = format!("{line}{word}");
        if text_width_mm(&candidate, size_pt) <= max_mm || line.is_empty() {
            if text_width_mm(&word, size_pt) > max_mm {
                // a single token wider than the line — break per character
                for ch in word.chars() {
                    let c2 = format!("{line}{ch}");
                    if text_width_mm(&c2, size_pt) > max_mm && !line.is_empty() {
                        lines.push(std::mem::take(&mut line));
                    }
                    line.push(ch);
                }
            } else {
                line.push_str(&word);
            }
        } else {
            lines.push(std::mem::take(&mut line));
            line.push_str(word.trim_start());
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn split_keep_spaces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        cur.push(c);
        if c == ' ' {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

// ───────────────────── layout engine ─────────────────────

/// Accumulates ops onto A4 pages, breaking to a new page when the cursor drops
/// past the bottom margin.
struct Flow {
    font: FontId,
    pages: Vec<Vec<Op>>,
    ops: Vec<Op>,
    y_mm: f32,
    in_text: bool,
}

impl Flow {
    fn new(font: FontId) -> Self {
        Flow {
            font,
            pages: Vec::new(),
            ops: Vec::new(),
            y_mm: PAGE_H_MM - MARGIN_MM,
            in_text: false,
        }
    }

    fn end_text(&mut self) {
        if self.in_text {
            self.ops.push(Op::EndTextSection);
            self.in_text = false;
        }
    }

    fn new_page(&mut self) {
        self.end_text();
        self.pages.push(std::mem::take(&mut self.ops));
        self.y_mm = PAGE_H_MM - MARGIN_MM;
    }

    fn ensure_space(&mut self, need_mm: f32) {
        if self.y_mm - need_mm < MARGIN_MM {
            self.new_page();
        }
    }

    /// Draw one already-wrapped line at `size_pt`, advancing the cursor.
    fn line(&mut self, text: &str, size_pt: f32) {
        let line_h_mm = size_pt * 0.352_777_8 * LEADING;
        self.ensure_space(line_h_mm);
        self.y_mm -= line_h_mm;
        self.ops.push(Op::StartTextSection);
        self.ops.push(Op::SetFont {
            font: PdfFontHandle::External(self.font.clone()),
            size: Pt(size_pt),
        });
        self.ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(MARGIN_MM), Mm(self.y_mm)),
        });
        self.ops.push(Op::ShowText {
            items: vec![TextItem::Text(text.to_string())],
        });
        self.ops.push(Op::EndTextSection);
    }

    fn paragraph(&mut self, text: &str, size_pt: f32) {
        for l in wrap(text, size_pt, CONTENT_W_MM) {
            self.line(&l, size_pt);
        }
    }

    fn gap(&mut self, mm: f32) {
        self.y_mm -= mm;
        if self.y_mm < MARGIN_MM {
            self.new_page();
        }
    }

    fn finish(mut self) -> Vec<Vec<Op>> {
        self.end_text();
        if !self.ops.is_empty() {
            self.pages.push(std::mem::take(&mut self.ops));
        }
        if self.pages.is_empty() {
            self.pages.push(Vec::new());
        }
        self.pages
    }
}

fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// `build_document` PDF.
pub fn render_document(req: &DocRequest) -> Result<Vec<u8>, AppError> {
    let face = Face::load()?;
    let mut doc = PdfDocument::new(req.title.trim());
    let font = doc.add_font(&face.parsed);
    let mut flow = Flow::new(font);

    flow.paragraph(req.title.trim(), SZ_TITLE);
    flow.gap(4.0);

    if req.toc {
        flow.paragraph("目次 / Contents", SZ_H[0]);
        flow.gap(1.0);
        for s in req.sections {
            if let Some(h) = &s.heading {
                let indent = "    ".repeat((s.level.clamp(1, 4) as usize).saturating_sub(1));
                flow.paragraph(&format!("{indent}• {}", h.trim()), SZ_BODY);
            }
        }
        flow.gap(3.0);
    }

    for s in req.sections {
        if let Some(h) = &s.heading {
            let idx = (s.level.clamp(1, 4) as usize) - 1;
            flow.gap(2.0);
            flow.paragraph(h.trim(), SZ_H[idx]);
            flow.gap(1.0);
        }
        for block in parse_blocks(&s.body) {
            match block {
                Block::Para(spans) => {
                    flow.paragraph(&spans_text(&spans), SZ_BODY);
                    flow.gap(1.5);
                }
                Block::Bullet(items) => {
                    for it in items {
                        flow.paragraph(&format!("•  {}", spans_text(&it)), SZ_BODY);
                    }
                    flow.gap(1.5);
                }
                Block::Number(items) => {
                    for (i, it) in items.iter().enumerate() {
                        flow.paragraph(&format!("{}.  {}", i + 1, spans_text(it)), SZ_BODY);
                    }
                    flow.gap(1.5);
                }
                Block::Code(text) => {
                    for l in text.lines() {
                        flow.line(l, SZ_CODE);
                    }
                    flow.gap(1.5);
                }
                Block::Table(rows) => {
                    for row in rows {
                        flow.paragraph(&row.join("    |    "), SZ_BODY);
                    }
                    flow.gap(1.5);
                }
            }
        }
    }

    let pages: Vec<PdfPage> = flow
        .finish()
        .into_iter()
        .map(|ops| PdfPage::new(Mm(PAGE_W_MM), Mm(PAGE_H_MM), ops))
        .collect();
    doc.with_pages(pages);
    let mut warn = Vec::new();
    Ok(doc.save(&PdfSaveOptions::default(), &mut warn))
}

// ───────────────────── OCR sandwich ─────────────────────

/// One page for [`render_sandwich`]: the rendered page raster (PNG/JPEG bytes)
/// plus OCR word/line boxes as page fractions.
pub struct SandwichPage {
    pub image: Vec<u8>,
    pub width_pt: f32,
    pub height_pt: f32,
    /// `(text, [x, y, w, h] in 0..1, from the top-left)`.
    pub lines: Vec<(String, [f32; 4])>,
}

/// Build `searchable.pdf`: each page is the raster at full size with an
/// invisible, position-matched OCR text layer over it (selectable, findable).
pub fn render_sandwich(pages: &[SandwichPage]) -> Result<Vec<u8>, AppError> {
    if pages.is_empty() {
        return Err(err("SANDWICH_EMPTY", "no pages"));
    }
    let face = Face::load()?;
    let mut doc = PdfDocument::new("searchable");
    let font = doc.add_font(&face.parsed);

    let mut pdf_pages = Vec::with_capacity(pages.len());
    for p in pages {
        let w_pt = p.width_pt.max(1.0);
        let h_pt = p.height_pt.max(1.0);
        let mut warn = Vec::new();
        let img = RawImage::decode_from_bytes(&p.image, &mut warn)
            .map_err(|e| err("SANDWICH_IMAGE", e))?;
        let img_id = doc.add_image(&img);

        let mut ops = vec![Op::UseXobject {
            id: img_id.clone(),
            transform: XObjectTransform {
                translate_x: Some(Pt(0.0)),
                translate_y: Some(Pt(0.0)),
                scale_x: Some(w_pt / img.width.max(1) as f32),
                scale_y: Some(h_pt / img.height.max(1) as f32),
                ..Default::default()
            },
        }];

        for (text, [fx, fy, _fw, fh]) in &p.lines {
            if text.trim().is_empty() {
                continue;
            }
            let size = (fh * h_pt).clamp(4.0, 48.0);
            // OCR y is from the top; PDF y is from the bottom. Place the baseline
            // near the bottom of the box.
            let x = fx * w_pt;
            let y = h_pt - (fy * h_pt) - size;
            ops.push(Op::StartTextSection);
            ops.push(Op::SetTextRenderingMode {
                mode: TextRenderingMode::Invisible,
            });
            ops.push(Op::SetFont {
                font: PdfFontHandle::External(font.clone()),
                size: Pt(size),
            });
            ops.push(Op::SetTextCursor {
                pos: Point { x: Pt(x), y: Pt(y) },
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(text.clone())],
            });
            ops.push(Op::EndTextSection);
        }

        pdf_pages.push(PdfPage::new(
            Mm(w_pt / 2.834_645_7),
            Mm(h_pt / 2.834_645_7),
            ops,
        ));
    }

    doc.with_pages(pdf_pages);
    let mut warn = Vec::new();
    Ok(doc.save(&PdfSaveOptions::default(), &mut warn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::doc_builder::DocSection;

    #[test]
    fn wrap_breaks_long_latin_and_cjk_lines() {
        let latin = wrap(
            "the quick brown fox jumps over the lazy dog again and again",
            10.5,
            40.0,
        );
        assert!(latin.len() > 1);
        assert!(latin.iter().all(|l| text_width_mm(l, 10.5) <= 40.5));

        let cjk = wrap(&"漢字".repeat(40), 10.5, 40.0);
        assert!(cjk.len() > 1);
    }

    #[test]
    fn render_document_produces_a_pdf_or_a_clean_unavailable_error() {
        let sections = vec![DocSection {
            level: 1,
            heading: Some("概要".into()),
            body: "本文の段落です。**強調**。\n\n- 箇条書き A\n- 箇条書き B\n\n```\ncode()\n```"
                .into(),
        }];
        let req = DocRequest {
            title: "四半期レポート",
            toc: true,
            sections: &sections,
        };
        match render_document(&req) {
            Ok(bytes) => {
                assert_eq!(&bytes[..5], b"%PDF-", "not a PDF header");
                assert!(bytes.len() > 800);
            }
            Err(e) => assert_eq!(e.code, "DOC_BUILD_PDF_UNAVAILABLE"),
        }
    }
}
