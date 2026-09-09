//! Imported / authored static-site support (issues 4 & 5).
//!
//! A "website" source is a **folder** of static files copied verbatim into
//! `sources/<id>/`. The Viewer previews it in a sandboxed `<iframe>` served
//! through the existing `wakaru-asset://` scheme (see `services::assets`), so
//! the copy never leaves the project sandbox and the frame gets its own opaque
//! origin. Ingest extracts readable text from every HTML file so Studio / Live
//! Illustrator can still reason about the site.

use crate::error::{AppError, AppResult};
use crate::services::ingest::Unit;
use std::path::{Component, Path, PathBuf};

/// Hard ceilings for one imported folder — a runaway `node_modules` or a
/// symlink loop can't blow up memory or disk.
pub const MAX_FILES: usize = 4_000;
pub const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_DEPTH: usize = 24;
/// HTML larger than this is copied but not text-extracted (keeps ingest bounded).
const MAX_HTML_PARSE_BYTES: u64 = 3 * 1024 * 1024;

/// Files we are willing to copy into the sandbox. Anything else (executables,
/// archives, unknown binaries) is skipped — the site still previews, it just
/// won't carry payloads we don't understand.
fn allowed_ext(ext: &str) -> bool {
    matches!(
        ext,
        "html"
            | "htm"
            | "css"
            | "js"
            | "mjs"
            | "cjs"
            | "json"
            | "map"
            | "wasm"
            | "svg"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "avif"
            | "ico"
            | "bmp"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
            | "txt"
            | "md"
            | "xml"
            | "csv"
            | "webmanifest"
            | "mp4"
            | "webm"
            | "ogg"
            | "mp3"
            | "wav"
            | "m4a"
            | "pdf"
    )
}

/// Directory names never worth importing.
fn skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "__pycache__" | ".git" | ".svn" | ".hg"
        )
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct WebsiteFile {
    /// POSIX-style path relative to the site root.
    pub path: String,
    #[ts(type = "number")]
    pub bytes: u64,
    pub is_entry: bool,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct WebsiteManifest {
    /// Entry document, relative to the site root (usually `index.html`).
    pub entry: String,
    pub files: Vec<WebsiteFile>,
}

/// Result of a safe folder copy.
pub struct Copied {
    /// Entry HTML, POSIX-relative to `dest_root`.
    pub entry: String,
    pub total_bytes: u64,
    pub file_count: usize,
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::new("WEBSITE_INVALID", "error.website.invalid", msg)
}

/// Copy every allowed file under `src_root` into `dest_root`, enforcing the
/// ceilings above. Symlinks are never followed. Returns the detected entry.
pub fn copy_site_tree(src_root: &Path, dest_root: &Path) -> AppResult<Copied> {
    let meta =
        std::fs::symlink_metadata(src_root).map_err(|e| bad(format!("cannot read folder: {e}")))?;
    if !meta.is_dir() {
        return Err(bad("not a folder"));
    }

    let mut htmls: Vec<String> = Vec::new();
    let mut total: u64 = 0;
    let mut count: usize = 0;

    // Iterative walk — no recursion, so a pathological tree can't overflow the
    // stack. Each entry carries its depth and POSIX-relative path.
    let mut stack: Vec<(PathBuf, String, usize)> = vec![(src_root.to_path_buf(), String::new(), 0)];
    while let Some((dir, rel, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            continue;
        }
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.is_empty() || name.contains('/') || name.contains('\\') {
                continue;
            }
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_symlink() {
                continue; // never follow — could escape the source folder
            }
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if ft.is_dir() {
                if skip_dir(&name) {
                    continue;
                }
                stack.push((entry.path(), child_rel, depth + 1));
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let ext = Path::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !allowed_ext(&ext) {
                continue;
            }
            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if count + 1 > MAX_FILES {
                return Err(bad(format!("folder has more than {MAX_FILES} files")));
            }
            if total + len > MAX_TOTAL_BYTES {
                return Err(bad(format!(
                    "folder exceeds {} MB",
                    MAX_TOTAL_BYTES / 1024 / 1024
                )));
            }

            let dest = dest_root.join(rel_to_native(&child_rel));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dest)?;
            total += len;
            count += 1;
            if ext == "html" || ext == "htm" {
                htmls.push(child_rel);
            }
        }
    }

    if htmls.is_empty() {
        return Err(bad("no .html file found in the folder"));
    }
    Ok(Copied {
        entry: pick_entry(&htmls),
        total_bytes: total,
        file_count: count,
    })
}

/// `index.html` at the root wins; else the shallowest, then lexicographically
/// first HTML file.
fn pick_entry(htmls: &[String]) -> String {
    if let Some(root_index) = htmls.iter().find(|h| h.eq_ignore_ascii_case("index.html")) {
        return root_index.clone();
    }
    let mut sorted: Vec<&String> = htmls.iter().collect();
    sorted.sort_by(|a, b| {
        (a.matches('/').count(), a.as_str()).cmp(&(b.matches('/').count(), b.as_str()))
    });
    sorted
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "index.html".to_string())
}

fn rel_to_native(rel: &str) -> PathBuf {
    rel.split('/').fold(PathBuf::new(), |mut acc, seg| {
        acc.push(seg);
        acc
    })
}

/// Reject a caller-supplied relative path that isn't a plain descendant.
pub fn safe_rel(rel: &str) -> AppResult<String> {
    let rel = rel.trim();
    if rel.is_empty() {
        return Err(bad("empty path"));
    }
    if rel.starts_with('/') || rel.contains('\\') || rel.contains(':') || rel.contains('%') {
        return Err(bad("path escapes the site"));
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(bad("path escapes the site"));
    }
    for c in p.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            _ => return Err(bad("path escapes the site")),
        }
    }
    Ok(rel.trim_start_matches("./").to_string())
}

/// Walk the copied site under `site_root` and list every file (POSIX-relative).
pub fn manifest(site_root: &Path, entry: &str) -> AppResult<WebsiteManifest> {
    let mut files: Vec<WebsiteFile> = Vec::new();
    let mut stack: Vec<(PathBuf, String, usize)> =
        vec![(site_root.to_path_buf(), String::new(), 0)];
    while let Some((dir, rel, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            continue;
        }
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for dent in read.flatten() {
            let name = dent.file_name().to_string_lossy().to_string();
            let ft = match dent.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_symlink() {
                continue;
            }
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if ft.is_dir() {
                stack.push((dent.path(), child_rel, depth + 1));
            } else if ft.is_file() {
                files.push(WebsiteFile {
                    bytes: dent.metadata().map(|m| m.len()).unwrap_or(0),
                    is_entry: child_rel == entry,
                    path: child_rel,
                });
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(WebsiteManifest {
        entry: entry.to_string(),
        files,
    })
}

/// Ingest text extraction for `SourceKind::Website`. `site_root` is the copied
/// `sources/<id>/` folder; `entry` is the entry HTML (POSIX-relative).
pub fn parse_site(site_root: &Path, entry: &str) -> AppResult<Vec<Unit>> {
    // Collect HTML files, entry first, then the rest sorted.
    let manifest = manifest(site_root, entry)?;
    let mut htmls: Vec<String> = manifest
        .files
        .iter()
        .map(|f| f.path.clone())
        .filter(|p| {
            let e = Path::new(p)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            e == "html" || e == "htm"
        })
        .collect();
    htmls.sort();
    htmls.sort_by_key(|p| (p != entry) as u8);

    let mut units = Vec::new();
    for (i, rel) in htmls.iter().enumerate() {
        let abs = site_root.join(rel_to_native(rel));
        let len = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
        let (title, body) = if len <= MAX_HTML_PARSE_BYTES {
            match std::fs::read_to_string(&abs) {
                Ok(html) => {
                    let (title, sections) = crate::services::ingest::web::extract_readable(&html);
                    let body = sections
                        .into_iter()
                        .map(|(h, b)| match h {
                            Some(h) => format!("## {h}\n\n{b}"),
                            None => b,
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    (title, body)
                }
                Err(_) => (None, String::new()),
            }
        } else {
            (None, String::new())
        };
        let heading = title.clone().unwrap_or_else(|| rel.clone());
        let text = format!("# {heading}\n\n(source file: {rel})\n\n{body}")
            .trim()
            .to_string();
        units.push(Unit {
            ordinal: (i + 1) as u32,
            kind: "page",
            title: Some(heading),
            text,
            locator: serde_json::json!({ "t": "file", "path": rel }),
        });
    }
    if units.is_empty() {
        return Err(bad("no readable HTML in the site"));
    }
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    #[test]
    fn copies_allowed_files_skips_junk_and_finds_entry() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        write(
            src.path(),
            "index.html",
            "<title>Home</title><h1>Hi</h1><p>Body.</p>",
        );
        write(
            src.path(),
            "about/index.html",
            "<title>About</title><p>About us.</p>",
        );
        write(src.path(), "css/site.css", "body{color:red}");
        write(src.path(), "node_modules/pkg/x.js", "danger()");
        write(src.path(), "run.exe", "MZ");
        let copied = copy_site_tree(src.path(), dst.path()).unwrap();
        assert_eq!(copied.entry, "index.html");
        assert!(dst.path().join("css/site.css").exists());
        assert!(!dst.path().join("node_modules/pkg/x.js").exists());
        assert!(!dst.path().join("run.exe").exists());
        assert_eq!(copied.file_count, 3);
    }

    #[test]
    fn entry_falls_back_to_shallowest_html() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        write(src.path(), "pages/a.html", "<p>a</p>");
        write(src.path(), "home.html", "<p>home</p>");
        let copied = copy_site_tree(src.path(), dst.path()).unwrap();
        assert_eq!(copied.entry, "home.html");
    }

    #[test]
    fn folder_without_html_is_rejected() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        write(src.path(), "data.json", "{}");
        assert!(copy_site_tree(src.path(), dst.path()).is_err());
    }

    #[test]
    fn parse_site_yields_a_unit_per_html_entry_first() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "index.html",
            "<title>Home</title><h1>Home</h1><p>Welcome.</p>",
        );
        write(
            root.path(),
            "a/b.html",
            "<title>Deep</title><p>Deep page.</p>",
        );
        let units = parse_site(root.path(), "index.html").unwrap();
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].locator["path"], "index.html");
        assert!(units[0].text.contains("Welcome."));
    }

    #[test]
    fn safe_rel_rejects_traversal() {
        assert!(safe_rel("../x").is_err());
        assert!(safe_rel("/etc/passwd").is_err());
        assert_eq!(safe_rel("a/b/c.html").unwrap(), "a/b/c.html");
    }
}
