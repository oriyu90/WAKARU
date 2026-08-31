//! Spreadsheet parsing (docs/04 §6). Phase 1: CSV / TSV only (pure Rust).
//! xlsx / xls (via `calamine`) land in the follow-up commit.

use super::Unit;
use crate::error::{AppError, AppResult};
use std::path::Path;

const BLOCK_ROWS: usize = 50;

pub fn parse_sheet(path: &Path) -> AppResult<Vec<Unit>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "csv" | "tsv" | "txt" => parse_delimited(path, if ext == "tsv" { b'\t' } else { b',' }),
        other => Err(AppError::new(
            "SOURCE_UNSUPPORTED_FORMAT",
            "error.source.unsupported",
            format!("spreadsheet format .{other} not supported yet (xlsx/xls in follow-up)"),
        )),
    }
}

fn parse_delimited(path: &Path, delim: u8) -> AppResult<Vec<Unit>> {
    let text = super::text::read_to_string(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim)
        .flexible(true)
        .has_headers(false)
        .from_reader(text.as_bytes());

    let rows: Vec<Vec<String>> = rdr
        .records()
        .filter_map(|r| r.ok())
        .map(|rec| rec.iter().map(|s| s.to_string()).collect())
        .collect();

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let header = &rows[0];
    let n_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut units = Vec::new();

    // A summary unit first (docs/04 §6: header + stats for large sheets).
    let mut summary = format!(
        "Sheet: {}\nRows: {}  Columns: {}\n\n",
        file_stem(path),
        rows.len(),
        n_cols
    );
    summary.push_str("Columns: ");
    summary.push_str(&header.join(", "));
    units.push(Unit {
        ordinal: 1,
        kind: "sheet",
        title: Some(file_stem(path)),
        text: summary,
        locator: serde_json::json!({ "t": "cell", "sheet": file_stem(path), "range": "A1:A1" }),
    });

    // Body as Markdown tables, 50-row blocks. (Sheets over 500 rows also keep the
    // full data in derived/tables/ — added in the follow-up commit.)
    let body_rows = &rows[..];
    let start_row = if header_is_names(header) { 1 } else { 0 };
    let mut ordinal = 2;
    let mut i = start_row;
    while i < body_rows.len() {
        let end = (i + BLOCK_ROWS).min(body_rows.len());
        let mut md = String::new();
        md.push_str(&md_row(header));
        md.push_str(&md_sep(header.len().max(1)));
        for row in &body_rows[i..end] {
            md.push_str(&md_row(row));
        }
        units.push(Unit {
            ordinal,
            kind: "sheet",
            title: Some(format!("rows {}–{}", i + 1, end)),
            text: md,
            locator: serde_json::json!({
                "t": "cell",
                "sheet": file_stem(path),
                "range": format!("A{}:{}{}", i + 1, col_letter(n_cols.max(1) - 1), end)
            }),
        });
        ordinal += 1;
        i = end;
        if ordinal > 400 {
            break; // safety
        }
    }
    Ok(units)
}

fn header_is_names(header: &[String]) -> bool {
    !header.is_empty()
        && header.iter().all(|c| !c.trim().is_empty())
        && header.iter().filter(|c| c.parse::<f64>().is_ok()).count() * 2 < header.len()
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
fn col_letter(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s
}
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sheet")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_produces_summary_plus_body() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("data.csv");
        std::fs::write(&p, "name,age\nAlice,30\nBob,25\n").unwrap();
        let u = parse_sheet(&p).unwrap();
        assert!(u.len() >= 2);
        assert!(u[0].text.contains("Columns: name, age"));
        assert!(u[1].text.contains("Alice"));
    }
}
