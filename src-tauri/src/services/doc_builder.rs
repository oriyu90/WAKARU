//! `build_document` — deterministic document assembly for the Studio tool of the
//! same name (P12). The model sends *structure* (title + sections of Markdown),
//! not a formatted document, so a weak model can't get the layout wrong and the
//! request stays small. Output: Markdown, Word (`docx-rs`) or PDF (`printpdf`
//! with a bundled CJK face — see `services::pdf_text`).

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct DocSection {
    /// 1..=4; clamped. Ignored when `heading` is None.
    pub level: u8,
    pub heading: Option<String>,
    /// Markdown subset: paragraphs, `-`/`*`/`1.` lists, fenced code, simple
    /// `|` tables, `**bold**`, `*italic*`, `` `code` ``.
    pub body: String,
}

pub struct DocRequest<'a> {
    pub title: &'a str,
    pub toc: bool,
    pub sections: &'a [DocSection],
}

/// Render to bytes. `format` is `"md" | "docx" | "pdf"`.
pub fn render(format: &str, req: &DocRequest) -> AppResult<Vec<u8>> {
    if req.title.trim().is_empty() {
        return Err(bad("title is required"));
    }
    if req.sections.is_empty() {
        return Err(bad("at least one section is required"));
    }
    match format {
        "md" | "markdown" => Ok(render_markdown(req).into_bytes()),
        "docx" => render_docx(req),
        "pdf" => crate::services::pdf_text::render_document(req),
        other => Err(bad(&format!("unsupported format: {other}"))),
    }
}

/// A hint appended to the tool result so the model can tell the reader why a
/// `pdf` request fell back. Empty when PDF is available.
pub fn pdf_note() -> &'static str {
    crate::services::pdf_text::AVAILABILITY_NOTE
}

fn bad(msg: &str) -> AppError {
    AppError::new("DOC_BUILD_INVALID", "errors.doc.invalid", msg)
}

fn lvl(section: &DocSection) -> usize {
    section.level.clamp(1, 4) as usize
}

// ───────────────────────── Markdown ─────────────────────────

fn render_markdown(req: &DocRequest) -> String {
    let mut out = format!("# {}\n\n", req.title.trim());
    if req.toc {
        out.push_str("## 目次 / Contents\n\n");
        for s in req.sections {
            if let Some(h) = &s.heading {
                let indent = "  ".repeat(lvl(s).saturating_sub(1));
                out.push_str(&format!("{indent}- {}\n", h.trim()));
            }
        }
        out.push('\n');
    }
    for s in req.sections {
        if let Some(h) = &s.heading {
            out.push_str(&format!("{} {}\n\n", "#".repeat(lvl(s) + 1), h.trim()));
        }
        let body = s.body.trim();
        if !body.is_empty() {
            out.push_str(body);
            out.push_str("\n\n");
        }
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

// ───────────────────────── DOCX ─────────────────────────

fn render_docx(req: &DocRequest) -> AppResult<Vec<u8>> {
    use docx_rs::*;

    let mut docx = Docx::new().add_paragraph(
        Paragraph::new()
            .style("Title")
            .add_run(Run::new().add_text(req.title.trim())),
    );

    if req.toc {
        docx = docx.add_paragraph(
            Paragraph::new()
                .style("Heading1")
                .add_run(Run::new().add_text("目次 / Contents")),
        );
        for s in req.sections {
            if let Some(h) = &s.heading {
                docx = docx.add_paragraph(
                    Paragraph::new()
                        .indent(Some(360 * (lvl(s) as i32 - 1)), None, None, None)
                        .add_run(Run::new().add_text(format!("• {}", h.trim()))),
                );
            }
        }
    }

    for s in req.sections {
        if let Some(h) = &s.heading {
            let style = format!("Heading{}", lvl(s).min(4));
            docx = docx.add_paragraph(
                Paragraph::new()
                    .style(&style)
                    .add_run(Run::new().add_text(h.trim())),
            );
        }
        for block in parse_blocks(&s.body) {
            docx = add_block_docx(docx, &block);
        }
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    docx.build()
        .pack(&mut buf)
        .map_err(|e| AppError::new("DOC_BUILD_DOCX", "errors.doc.build", e.to_string()))?;
    Ok(buf.into_inner())
}

fn add_block_docx(docx: docx_rs::Docx, block: &Block) -> docx_rs::Docx {
    use docx_rs::*;
    match block {
        Block::Para(spans) => docx.add_paragraph(spans_to_para(Paragraph::new(), spans)),
        Block::Bullet(items) => items.iter().fold(docx, |d, spans| {
            d.add_paragraph(spans_to_para(
                Paragraph::new().add_run(Run::new().add_text("•  ")),
                spans,
            ))
        }),
        Block::Number(items) => items.iter().enumerate().fold(docx, |d, (i, spans)| {
            d.add_paragraph(spans_to_para(
                Paragraph::new().add_run(Run::new().add_text(format!("{}.  ", i + 1))),
                spans,
            ))
        }),
        Block::Code(text) => text.lines().fold(docx, |d, line| {
            d.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .fonts(RunFonts::new().ascii("Consolas"))
                        .add_text(line),
                ),
            )
        }),
        Block::Table(rows) => rows.iter().fold(docx, |d, row| {
            d.add_paragraph(Paragraph::new().add_run(Run::new().add_text(row.join("    "))))
        }),
    }
}

fn spans_to_para(mut p: docx_rs::Paragraph, spans: &[Span]) -> docx_rs::Paragraph {
    use docx_rs::*;
    for span in spans {
        let mut run = Run::new().add_text(&span.text);
        if span.bold {
            run = run.bold();
        }
        if span.italic {
            run = run.italic();
        }
        if span.code {
            run = run.fonts(RunFonts::new().ascii("Consolas"));
        }
        p = p.add_run(run);
    }
    p
}

// ───────────────────────── Markdown subset parser ─────────────────────────

#[derive(Debug, Clone)]
pub enum Block {
    Para(Vec<Span>),
    Bullet(Vec<Vec<Span>>),
    Number(Vec<Vec<Span>>),
    Code(String),
    Table(Vec<Vec<String>>),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

/// Small, forgiving block parser. Unrecognised syntax degrades to a paragraph.
pub fn parse_blocks(body: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = body.lines().peekable();
    while let Some(&line) = lines.peek() {
        let t = line.trim_end();
        if t.trim().is_empty() {
            lines.next();
            continue;
        }
        if t.trim_start().starts_with("```") {
            lines.next(); // opening fence
            let mut code = String::new();
            for l in lines.by_ref() {
                if l.trim_start().starts_with("```") {
                    break;
                }
                code.push_str(l);
                code.push('\n');
            }
            blocks.push(Block::Code(code.trim_end().to_string()));
            continue;
        }
        if is_bullet(t) {
            let mut items = Vec::new();
            while lines
                .peek()
                .map(|l| is_bullet(l.trim_end()))
                .unwrap_or(false)
            {
                let l = lines.next().unwrap();
                items.push(parse_spans(strip_bullet(l.trim_end())));
            }
            blocks.push(Block::Bullet(items));
            continue;
        }
        if is_numbered(t) {
            let mut items = Vec::new();
            while lines
                .peek()
                .map(|l| is_numbered(l.trim_end()))
                .unwrap_or(false)
            {
                let l = lines.next().unwrap();
                items.push(parse_spans(strip_numbered(l.trim_end())));
            }
            blocks.push(Block::Number(items));
            continue;
        }
        if is_table_row(t) {
            let mut rows = Vec::new();
            while lines
                .peek()
                .map(|l| is_table_row(l.trim_end()))
                .unwrap_or(false)
            {
                let l = lines.next().unwrap();
                let cells: Vec<String> = l
                    .trim()
                    .trim_matches('|')
                    .split('|')
                    .map(|c| c.trim().to_string())
                    .collect();
                if cells
                    .iter()
                    .all(|c| c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
                {
                    continue; // separator row
                }
                rows.push(cells);
            }
            if !rows.is_empty() {
                blocks.push(Block::Table(rows));
            }
            continue;
        }
        // paragraph: consume until a blank line
        let mut para = String::new();
        while let Some(&l) = lines.peek() {
            if l.trim().is_empty()
                || is_bullet(l)
                || is_numbered(l)
                || l.trim_start().starts_with("```")
            {
                break;
            }
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(l.trim());
            lines.next();
        }
        blocks.push(Block::Para(parse_spans(&para)));
    }
    blocks
}

fn is_bullet(l: &str) -> bool {
    let t = l.trim_start();
    t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")
}
fn strip_bullet(l: &str) -> &str {
    l.trim_start().get(2..).unwrap_or("")
}
fn is_numbered(l: &str) -> bool {
    let t = l.trim_start();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && t[digits.len()..].starts_with(". ")
}
fn strip_numbered(l: &str) -> &str {
    let t = l.trim_start();
    let n = t.find(". ").map(|i| i + 2).unwrap_or(0);
    &t[n..]
}
fn is_table_row(l: &str) -> bool {
    let t = l.trim();
    t.starts_with('|') && t.matches('|').count() >= 2
}

/// Inline `**bold**`, `*italic*` / `_italic_`, `` `code` ``. An opening marker
/// with no close is emitted literally (no text is lost).
pub fn parse_spans(text: &str) -> Vec<Span> {
    let ch: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut cur = Span::default();
    let mut i = 0;
    let flush = |spans: &mut Vec<Span>, cur: &mut Span| {
        if !cur.text.is_empty() {
            spans.push(std::mem::take(cur));
        }
    };
    while i < ch.len() {
        let c = ch[i];
        // `code`
        if c == '`' {
            if let Some(end) = find_char(&ch, i + 1, '`') {
                flush(&mut spans, &mut cur);
                spans.push(Span {
                    text: ch[i + 1..end].iter().collect(),
                    code: true,
                    ..Default::default()
                });
                i = end + 1;
                continue;
            }
        }
        // **bold**
        if c == '*' && ch.get(i + 1) == Some(&'*') {
            if let Some(end) = find_str(&ch, i + 2, &['*', '*']) {
                flush(&mut spans, &mut cur);
                let inner: String = ch[i + 2..end].iter().collect();
                for s in parse_spans(&inner) {
                    spans.push(Span { bold: true, ..s });
                }
                i = end + 2;
                continue;
            }
        }
        // *italic* / _italic_
        if (c == '*' || c == '_') && ch.get(i + 1) != Some(&c) {
            if let Some(end) = find_char(&ch, i + 1, c) {
                flush(&mut spans, &mut cur);
                let inner: String = ch[i + 1..end].iter().collect();
                for s in parse_spans(&inner) {
                    spans.push(Span { italic: true, ..s });
                }
                i = end + 1;
                continue;
            }
        }
        cur.text.push(c);
        i += 1;
    }
    flush(&mut spans, &mut cur);
    if spans.is_empty() {
        spans.push(Span::default());
    }
    spans
}

fn find_char(ch: &[char], from: usize, target: char) -> Option<usize> {
    (from..ch.len()).find(|&j| ch[j] == target)
}

fn find_str(ch: &[char], from: usize, target: &[char]) -> Option<usize> {
    (from..ch.len().saturating_sub(target.len() - 1)).find(|&j| ch[j..j + target.len()] == *target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(level: u8, heading: &str, body: &str) -> DocSection {
        DocSection {
            level,
            heading: Some(heading.into()),
            body: body.into(),
        }
    }

    #[test]
    fn markdown_has_title_toc_and_levelled_headings() {
        let sections = vec![sec(1, "概要", "本文です。"), sec(2, "詳細", "- a\n- b")];
        let req = DocRequest {
            title: "四半期レポート",
            toc: true,
            sections: &sections,
        };
        let md = render_markdown(&req);
        assert!(md.starts_with("# 四半期レポート\n"));
        assert!(md.contains("## 目次"));
        assert!(md.contains("\n## 概要\n"));
        assert!(md.contains("\n### 詳細\n"));
        assert!(md.contains("- a\n- b"));
    }

    #[test]
    fn span_parser_handles_bold_italic_code_and_unbalanced_markers() {
        let s = parse_spans("plain **bold** and *it* and `x` and *unbalanced");
        let joined: String = s.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(joined, "plain bold and it and x and *unbalanced");
        assert!(s.iter().any(|x| x.bold && x.text == "bold"));
        assert!(s.iter().any(|x| x.italic && x.text == "it"));
        assert!(s.iter().any(|x| x.code && x.text == "x"));
        // the trailing "*unbalanced" is literal — no span carries it as italic
        assert!(!s.iter().any(|x| x.italic && x.text.contains("unbalanced")));
    }

    #[test]
    fn block_parser_classifies_lists_code_tables_and_paragraphs() {
        let blocks = parse_blocks(
            "A paragraph\nsecond line.\n\n- one\n- two\n\n1. first\n2. second\n\n```\ncode()\n```\n\n| a | b |\n| - | - |\n| 1 | 2 |",
        );
        assert!(matches!(blocks[0], Block::Para(_)));
        assert!(matches!(&blocks[1], Block::Bullet(v) if v.len() == 2));
        assert!(matches!(&blocks[2], Block::Number(v) if v.len() == 2));
        assert!(matches!(&blocks[3], Block::Code(c) if c == "code()"));
        // header + body row kept, separator row dropped
        assert!(
            matches!(&blocks[4], Block::Table(r) if r.len() == 2 && r[0] == vec!["a", "b"] && r[1] == vec!["1", "2"])
        );
    }

    #[test]
    fn docx_is_a_valid_zip_with_document_xml() {
        let sections = vec![sec(1, "Section", "Body **bold**.\n\n- item")];
        let bytes = render(
            "docx",
            &DocRequest {
                title: "Report",
                toc: false,
                sections: &sections,
            },
        )
        .unwrap();
        // .docx is a zip; the OOXML part must be present.
        assert_eq!(&bytes[..2], b"PK");
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        names.sort();
        assert!(names.iter().any(|n| n == "word/document.xml"), "{names:?}");
    }

    #[test]
    fn render_rejects_empty_title_or_sections() {
        assert!(render(
            "md",
            &DocRequest {
                title: "  ",
                toc: false,
                sections: &[sec(1, "h", "b")]
            }
        )
        .is_err());
        assert!(render(
            "md",
            &DocRequest {
                title: "T",
                toc: false,
                sections: &[]
            }
        )
        .is_err());
        assert!(render(
            "rtf",
            &DocRequest {
                title: "T",
                toc: false,
                sections: &[sec(1, "h", "b")]
            }
        )
        .is_err());
    }
}
