//! `wakaru-asset://` — the only way the WebView reads project files. The URL is
//! opaque (`wakaru-asset://localhost/<projectId>/<sourceId>/<rel>`); the handler
//! resolves it under `projects/<pid>/`, canonicalises, and refuses anything that
//! escapes the project's `sources/` or `derived/` (docs/02 §2 — no raw paths to
//! the front-end, no arbitrary file reads).

use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

pub const SCHEME: &str = "wakaru-asset";

/// Build the URL the front-end will use in `<img src>` / `<video src>` / fetch.
pub fn url(project_id: &str, source_id: &str, rel: &str) -> String {
    let rel = rel.trim_start_matches('/');
    let enc: String = rel
        .split('/')
        .map(urlencoding_encode)
        .collect::<Vec<_>>()
        .join("/");
    format!("{SCHEME}://localhost/{project_id}/{source_id}/{enc}")
}

/// Resolve a request path (`/<projectId>/<sourceId>/<rel...>`) to a real file,
/// or an error. `projects_root` is the app's `projects/` directory.
pub fn resolve(projects_root: &Path, request_path: &str) -> AppResult<PathBuf> {
    let parts: Vec<String> = request_path
        .trim_start_matches('/')
        .split('/')
        .map(urlencoding_decode)
        .collect();
    if parts.len() < 3 {
        return Err(deny("malformed asset path"));
    }
    let project_id = &parts[0];
    let source_id = &parts[1];
    if !safe_id(project_id) || !safe_id(source_id) {
        return Err(deny("invalid project or source id"));
    }
    let rel = parts[2..].join("/");

    // Only these two trees are ever readable.
    let project_dir = projects_root.join(project_id);
    let allowed_roots = [project_dir.join("sources"), project_dir.join("derived")];

    let candidate = if rel.starts_with("sources/") || rel.starts_with("derived/") {
        project_dir.join(&rel)
    } else {
        // default: look under both roots, scoped to this source id
        allowed_roots
            .iter()
            .map(|r| r.join(source_id).join(&rel))
            .find(|p| p.exists())
            .ok_or_else(|| deny("asset not found"))?
    };

    let real = std::fs::canonicalize(&candidate).map_err(|_| deny("asset not found"))?;
    let ok = allowed_roots.iter().any(|root| {
        std::fs::canonicalize(root)
            .map(|r| real.starts_with(&r))
            .unwrap_or(false)
    });
    if !ok {
        return Err(deny("asset path escapes the project sandbox"));
    }
    Ok(real)
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" | "aac" => "audio/aac",
        "flac" => "audio/flac",
        "ogg" | "opus" => "audio/ogg",
        "pdf" => "application/pdf",
        "md" | "markdown" => "text/markdown; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "txt" | "csv" | "tsv" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn deny(msg: &str) -> AppError {
    AppError::new("ASSET_DENIED", "error.asset.denied", msg)
}

// Tiny percent-codec — avoids pulling `urlencoding` for two call sites.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn urlencoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_roundtrips_and_encodes_spaces() {
        let u = url("pid", "sid", "pages/0001 copy.png");
        assert_eq!(u, "wakaru-asset://localhost/pid/sid/pages/0001%20copy.png");
    }

    #[test]
    fn resolve_rejects_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("p1/sources/s1")).unwrap();
        std::fs::write(root.join("p1/sources/s1/a.txt"), b"hi").unwrap();
        std::fs::write(root.join("secret.txt"), b"nope").unwrap();

        assert!(resolve(root, "/p1/s1/a.txt").is_ok());
        assert!(resolve(root, "/p1/s1/../../secret.txt").is_err());
        assert!(resolve(root, "/p1/s1/../../../secret.txt").is_err());
        assert!(resolve(root, "/../p1/s1/a.txt").is_err());
        assert!(resolve(root, "/p1/../s1/a.txt").is_err());
    }
}
