//! Text-family parsing (docs/04 §7). md/txt/code by section or block; JSON by
//! top-level key/element; JSONL by line. Encoding is sniffed, not assumed.

use super::Unit;
use crate::domain::source::SourceKind;
use crate::error::{AppError, AppResult};
use std::path::Path;

const BLOCK_CHARS: usize = 1500;

pub fn read_to_string(path: &Path) -> AppResult<String> {
    let bytes = std::fs::read(path)?;
    // BOM / heuristic charset detection (docs/04 §7).
    let mut det = chardetng::EncodingDetector::new();
    det.feed(&bytes, true);
    let enc = det.guess(None, true);
    let (text, _, _) = enc.decode(&bytes);
    Ok(text.into_owned())
}

pub fn parse_text(kind: SourceKind, path: &Path) -> AppResult<Vec<Unit>> {
    let text = read_to_string(path)?;
    match kind {
        SourceKind::Markdown => Ok(parse_markdown(&text)),
        SourceKind::Code => Ok(parse_blocks(&text, "section")),
        _ => Ok(parse_blocks(&text, "section")),
    }
}

/// Split Markdown on H1–H3. No headings → 1500-char blocks.
fn parse_markdown(text: &str) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut ordinal = 1;
    let mut cur_title: Option<String> = None;
    let mut buf = String::new();

    let flush = |units: &mut Vec<Unit>, ordinal: &mut u32, title: &Option<String>, buf: &mut String| {
        let body = buf.trim();
        if body.is_empty() && title.is_none() {
            buf.clear();
            return;
        }
        units.push(Unit {
            ordinal: *ordinal,
            kind: "section",
            title: title.clone(),
            text: if title.is_some() && !body.is_empty() {
                format!("# {}\n\n{}", title.as_deref().unwrap_or(""), body)
            } else {
                body.to_string()
            },
            locator: serde_json::json!({ "t": "line" }),
        });
        *ordinal += 1;
        buf.clear();
    };

    for line in text.lines() {
        let heading = line
            .strip_prefix("### ")
            .or_else(|| line.strip_prefix("## "))
            .or_else(|| line.strip_prefix("# "));
        if let Some(h) = heading {
            flush(&mut units, &mut ordinal, &cur_title, &mut buf);
            cur_title = Some(h.trim().to_string());
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    flush(&mut units, &mut ordinal, &cur_title, &mut buf);

    if units.is_empty() {
        return parse_blocks(text, "section");
    }
    units
}

/// Blank-line groups, capped at BLOCK_CHARS.
fn parse_blocks(text: &str, kind: &'static str) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut ordinal = 1;
    let mut cur = String::new();

    let push = |cur: &mut String, ordinal: &mut u32, units: &mut Vec<Unit>| {
        let t = cur.trim();
        if t.is_empty() {
            cur.clear();
            return;
        }
        units.push(Unit {
            ordinal: *ordinal,
            kind,
            title: None,
            text: t.to_string(),
            locator: serde_json::json!({ "t": "line" }),
        });
        *ordinal += 1;
        cur.clear();
    };

    for para in text.split("\n\n") {
        if cur.len() + para.len() > BLOCK_CHARS && !cur.is_empty() {
            push(&mut cur, &mut ordinal, &mut units);
        }
        if !cur.is_empty() {
            cur.push_str("\n\n");
        }
        cur.push_str(para);
        while cur.len() > BLOCK_CHARS {
            let cut = floor_char_boundary(&cur, BLOCK_CHARS);
            let rest = cur.split_off(cut);
            push(&mut cur, &mut ordinal, &mut units);
            cur = rest;
        }
    }
    push(&mut cur, &mut ordinal, &mut units);
    units
}

pub fn parse_json(path: &Path) -> AppResult<Vec<Unit>> {
    let text = read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        AppError::new("SOURCE_PARSE", "error.source.parse", format!("invalid JSON: {e}"))
    })?;
    let mut units = Vec::new();
    match value {
        serde_json::Value::Array(items) => {
            for (i, item) in items.into_iter().enumerate() {
                units.push(Unit {
                    ordinal: (i + 1) as u32,
                    kind: "record",
                    title: Some(format!("[{i}]")),
                    text: serde_json::to_string_pretty(&item).unwrap_or_default(),
                    locator: serde_json::json!({ "t": "path", "pointer": format!("/{i}") }),
                });
            }
        }
        serde_json::Value::Object(map) => {
            for (i, (k, v)) in map.into_iter().enumerate() {
                units.push(Unit {
                    ordinal: (i + 1) as u32,
                    kind: "record",
                    title: Some(k.clone()),
                    text: serde_json::to_string_pretty(&v).unwrap_or_default(),
                    locator: serde_json::json!({ "t": "path", "pointer": format!("/{k}") }),
                });
            }
        }
        other => units.push(Unit {
            ordinal: 1,
            kind: "record",
            title: None,
            text: other.to_string(),
            locator: serde_json::json!({ "t": "whole" }),
        }),
    }
    Ok(units)
}

pub fn parse_jsonl(path: &Path) -> AppResult<Vec<Unit>> {
    let text = read_to_string(path)?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let group = if lines.len() > 10_000 { 100 } else { 1 };
    let mut units = Vec::new();
    for (gi, batch) in lines.chunks(group).enumerate() {
        units.push(Unit {
            ordinal: (gi + 1) as u32,
            kind: "record",
            title: if group == 1 { None } else { Some(format!("lines {}–{}", gi * group + 1, gi * group + batch.len())) },
            text: batch.join("\n"),
            locator: serde_json::json!({ "t": "line", "start": gi * group + 1, "end": gi * group + batch.len() }),
        });
    }
    Ok(units)
}

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_splits_on_headings() {
        let md = "# One\n\nbody one\n\n## Two\n\nbody two";
        let u = parse_markdown(md);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].title.as_deref(), Some("One"));
        assert!(u[1].text.contains("body two"));
    }

    #[test]
    fn jsonl_one_unit_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.jsonl");
        std::fs::write(&p, "{\"a\":1}\n{\"a\":2}\n").unwrap();
        let u = parse_jsonl(&p).unwrap();
        assert_eq!(u.len(), 2);
    }

    #[test]
    fn json_array_element_per_unit() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.json");
        std::fs::write(&p, "[1,2,3]").unwrap();
        let u = parse_json(&p).unwrap();
        assert_eq!(u.len(), 3);
    }
}
