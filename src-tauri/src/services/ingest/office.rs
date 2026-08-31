//! OOXML parsing without a full office engine (docs/04 §4). pptx: one unit per
//! slide, speaker notes appended. docx: split on Heading 1–3, else one section.
//! xlsx/xls: delegated to `calamine`, one unit per sheet + 50-row blocks.

use super::Unit;
use crate::domain::source::SourceKind;
use crate::error::{AppError, AppResult};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::Read;
use std::path::Path;

pub fn parse_office(kind: SourceKind, path: &Path) -> AppResult<Vec<Unit>> {
    match kind {
        SourceKind::Slides => parse_pptx(path),
        SourceKind::Doc => parse_docx(path),
        SourceKind::Sheet => parse_xlsx(path),
        _ => Err(AppError::new(
            "SOURCE_UNSUPPORTED_FORMAT",
            "error.source.unsupported",
            "not office",
        )),
    }
}

fn open_zip(path: &Path) -> AppResult<zip::ZipArchive<std::fs::File>> {
    let f = std::fs::File::open(path)?;
    zip::ZipArchive::new(f).map_err(|e| {
        AppError::new(
            "SOURCE_PARSE",
            "error.source.parse",
            format!("not a valid OOXML file: {e}"),
        )
    })
}

fn entry_text(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut buf = String::new();
    zip.by_name(name).ok()?.read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// Collect the text of every `<tag>` element (namespace-insensitive: matches the
/// local name), joining with spaces / newlines.
fn collect_tag_text(xml: &str, local: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut out = String::new();
    let mut capture = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == local.as_bytes() => {
                capture = true;
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == local.as_bytes() => {
                capture = false;
                out.push('\n');
            }
            Ok(Event::Text(t)) if capture => {
                if let Ok(s) = t.unescape() {
                    out.push_str(&s);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    out
}

fn local_name(qname: &[u8]) -> &[u8] {
    match qname.iter().position(|&b| b == b':') {
        Some(i) => &qname[i + 1..],
        None => qname,
    }
}

fn parse_pptx(path: &Path) -> AppResult<Vec<Unit>> {
    let mut zip = open_zip(path)?;
    let slide_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    let mut slide_names = slide_names;
    slide_names.sort_by_key(|n| slide_num(n));

    let mut units = Vec::new();
    for (idx, name) in slide_names.iter().enumerate() {
        let n = slide_num(name);
        let body = entry_text(&mut zip, name)
            .map(|xml| collect_tag_text(&xml, "t"))
            .unwrap_or_default();
        let notes = entry_text(&mut zip, &format!("ppt/notesSlides/notesSlide{n}.xml"))
            .map(|xml| collect_tag_text(&xml, "t"))
            .unwrap_or_default();

        let mut text = format!("[スライド {}]\n{}", n, body.trim());
        if !notes.trim().is_empty() {
            text.push_str(&format!("\n\nノート: {}", notes.trim()));
        }
        units.push(Unit {
            ordinal: (idx + 1) as u32,
            kind: "slide",
            title: Some(format!("Slide {n}")),
            text,
            locator: serde_json::json!({ "t": "page", "page": n }),
        });
    }
    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "no slides",
        ));
    }
    Ok(units)
}

fn slide_num(name: &str) -> u32 {
    name.trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn parse_docx(path: &Path) -> AppResult<Vec<Unit>> {
    let mut zip = open_zip(path)?;
    let xml = entry_text(&mut zip, "word/document.xml").ok_or_else(|| {
        AppError::new("SOURCE_PARSE", "error.source.parse", "no word/document.xml")
    })?;

    // Walk paragraphs; a paragraph whose pStyle val starts with "Heading" opens
    // a new section (docs/04 §4 — DOCX has no stable page concept).
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut sections: Vec<(Option<String>, String)> = vec![(None, String::new())];
    let mut in_p = false;
    let mut p_text = String::new();
    let mut p_heading: Option<u8> = None;
    let mut capture_t = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match local_name(e.name().as_ref()) {
                b"p" => {
                    in_p = true;
                    p_text.clear();
                    p_heading = None;
                }
                b"pStyle" => {
                    if let Some(v) = e
                        .attributes()
                        .flatten()
                        .find(|a| local_name(a.key.as_ref()) == b"val")
                    {
                        if let Ok(val) = v.unescape_value() {
                            if let Some(rest) = val.strip_prefix("Heading") {
                                p_heading = rest.trim().parse::<u8>().ok();
                            }
                        }
                    }
                }
                b"t" => capture_t = true,
                _ => {}
            },
            Ok(Event::Empty(e)) if local_name(e.name().as_ref()) == b"pStyle" => {
                if let Some(v) = e
                    .attributes()
                    .flatten()
                    .find(|a| local_name(a.key.as_ref()) == b"val")
                {
                    if let Ok(val) = v.unescape_value() {
                        if let Some(rest) = val.strip_prefix("Heading") {
                            p_heading = rest.trim().parse::<u8>().ok();
                        }
                    }
                }
            }
            Ok(Event::Text(t)) if capture_t => {
                if let Ok(s) = t.unescape() {
                    p_text.push_str(&s);
                }
            }
            Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                b"t" => capture_t = false,
                b"p" => {
                    in_p = false;
                    let line = p_text.trim().to_string();
                    if !line.is_empty() {
                        if matches!(p_heading, Some(1..=3)) {
                            sections.push((Some(line.clone()), String::new()));
                        } else {
                            let last = sections.last_mut().unwrap();
                            last.1.push_str(&line);
                            last.1.push_str("\n\n");
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        let _ = in_p;
    }

    let mut units = Vec::new();
    let mut ordinal = 1;
    for (title, body) in sections {
        if body.trim().is_empty() && title.is_none() {
            continue;
        }
        let text = match &title {
            Some(t) => format!("# {t}\n\n{}", body.trim()),
            None => body.trim().to_string(),
        };
        if text.is_empty() {
            continue;
        }
        units.push(Unit {
            ordinal,
            kind: "section",
            title,
            text,
            locator: serde_json::json!({ "t": "line" }),
        });
        ordinal += 1;
    }
    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "empty document",
        ));
    }
    Ok(units)
}

fn parse_xlsx(path: &Path) -> AppResult<Vec<Unit>> {
    use calamine::{open_workbook_auto, Reader as _};
    let mut wb = open_workbook_auto(path).map_err(|e| {
        AppError::new(
            "SOURCE_PARSE",
            "error.source.parse",
            format!("cannot open spreadsheet: {e}"),
        )
    })?;

    let mut units = Vec::new();
    let mut ordinal = 1;
    let sheet_names = wb.sheet_names().to_vec();
    for sheet in sheet_names {
        let range = match wb.worksheet_range(&sheet) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|r| r.iter().map(cell_to_string).collect())
            .collect();
        if rows.is_empty() {
            continue;
        }
        let header = rows[0].clone();
        units.push(Unit {
            ordinal,
            kind: "sheet",
            title: Some(sheet.clone()),
            text: format!(
                "Sheet: {sheet}\nRows: {}  Columns: {}\nColumns: {}",
                rows.len(),
                header.len(),
                header.join(", ")
            ),
            locator: serde_json::json!({ "t": "cell", "sheet": sheet, "range": "A1:A1" }),
        });
        ordinal += 1;

        let mut i = 1.min(rows.len());
        while i < rows.len() {
            let end = (i + 50).min(rows.len());
            let mut md = md_row(&header);
            md.push_str(&md_sep(header.len().max(1)));
            for row in &rows[i..end] {
                md.push_str(&md_row(row));
            }
            units.push(Unit {
                ordinal,
                kind: "sheet",
                title: Some(format!("{sheet} rows {}–{}", i + 1, end)),
                text: md,
                locator: serde_json::json!({
                    "t": "cell", "sheet": sheet, "range": format!("A{}:Z{}", i + 1, end)
                }),
            });
            ordinal += 1;
            i = end;
            if ordinal > 500 {
                break;
            }
        }
    }
    if units.is_empty() {
        return Err(AppError::new(
            "SOURCE_EMPTY",
            "error.source.empty",
            "empty spreadsheet",
        ));
    }
    Ok(units)
}

fn cell_to_string(c: &calamine::Data) -> String {
    use calamine::Data as D;
    match c {
        D::Empty => std::string::String::new(),
        D::String(s) => s.clone(),
        D::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        D::Int(i) => i.to_string(),
        D::Bool(b) => b.to_string(),
        D::DateTime(d) => d.to_string(),
        D::DateTimeIso(s) => s.clone(),
        D::DurationIso(s) => s.clone(),
        D::Error(e) => format!("#ERR({e:?})"),
    }
}

fn md_row(cells: &[String]) -> String {
    format!(
        "| {} |\n",
        cells
            .iter()
            .map(|c| c.replace('|', "\\|"))
            .collect::<Vec<_>>()
            .join(" | ")
    )
}
fn md_sep(n: usize) -> String {
    format!("|{}\n", " --- |".repeat(n.max(1)))
}
