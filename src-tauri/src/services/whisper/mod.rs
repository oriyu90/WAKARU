//! Whisper model manager + the transcription entry point used by ingest
//! (docs/04 §2, docs/05, AC-5-*). Models are ggml files from the whisper.cpp
//! Hugging Face repo, downloaded on demand with HTTP-Range resume; the file's
//! SHA-256 is recorded on first success (DECISIONS D-15). Only one transcription
//! runs at a time, process-wide (AC-5-10).

pub mod audio;
pub mod engine;

use crate::domain::transcription::{DiskCheck, TranscriptSegment, WhisperModel};
use crate::error::{AppError, AppResult};
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct Catalog {
    name: &'static str,
    file: &'static str,
    url: &'static str,
    approx_bytes: u64,
}

const CATALOG: &[Catalog] = &[
    Catalog {
        name: "base",
        file: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        approx_bytes: 148 * 1024 * 1024,
    },
    Catalog {
        name: "small",
        file: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        approx_bytes: 488 * 1024 * 1024,
    },
    Catalog {
        name: "medium",
        file: "ggml-medium.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        approx_bytes: 1536 * 1024 * 1024,
    },
    Catalog {
        name: "large-v2",
        file: "ggml-large-v2.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v2.bin",
        approx_bytes: 3100u64 * 1024 * 1024,
    },
];

/// Names with a pending cancel request (set by `whisper_cancel_download`).
static CANCELS: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);

pub fn request_cancel(name: &str) {
    CANCELS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(Default::default)
        .insert(name.to_string());
}
pub fn is_cancel_requested(name: &str) -> bool {
    CANCELS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_some_and(|s| s.contains(name))
}
pub fn clear_cancel(name: &str) {
    if let Some(s) = CANCELS.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        s.remove(name);
    }
}

fn entry(name: &str) -> AppResult<&'static Catalog> {
    CATALOG
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| AppError::new("WHISPER_UNKNOWN_MODEL", "error.whisper.unknownModel", name))
}

pub fn models_root(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("whisper")
}

pub fn model_path(data_dir: &Path, name: &str) -> AppResult<PathBuf> {
    Ok(models_root(data_dir).join(entry(name)?.file))
}

/// Merge the static catalog with what's on disk and in `whisper_models`.
pub fn list(app_db: &Connection, data_dir: &Path, selected: &str) -> AppResult<Vec<WhisperModel>> {
    let root = models_root(data_dir);
    CATALOG
        .iter()
        .map(|c| {
            let path = root.join(c.file);
            let downloaded = path.is_file();
            let sha256: Option<String> = app_db
                .query_row(
                    "SELECT sha256 FROM whisper_models WHERE name = ?1",
                    [c.name],
                    |r| r.get(0),
                )
                .optional()?
                .flatten();
            Ok(WhisperModel {
                name: c.name.to_string(),
                approx_bytes: c.approx_bytes,
                downloaded,
                sha256: if downloaded { sha256 } else { None },
                path: downloaded.then(|| path.to_string_lossy().to_string()),
                selected: c.name == selected,
            })
        })
        .collect()
}

/// Free space vs. what the model needs plus 20% headroom (AC-5-2).
pub fn disk_check(data_dir: &Path, name: &str) -> AppResult<DiskCheck> {
    let needed = entry(name)?.approx_bytes + entry(name)?.approx_bytes / 5;
    let root = models_root(data_dir);
    std::fs::create_dir_all(&root)?;
    let free = free_bytes(&root).unwrap_or(u64::MAX);
    Ok(DiskCheck {
        ok: free >= needed,
        needed_bytes: needed,
        free_bytes: free,
    })
}

#[cfg(unix)]
fn free_bytes(path: &Path) -> Option<u64> {
    let s = nix::sys::statvfs::statvfs(path).ok()?;
    Some(s.blocks_available() as u64 * s.fragment_size() as u64)
}
#[cfg(not(unix))]
fn free_bytes(_path: &Path) -> Option<u64> {
    None
}

/// Download `name`, resuming a partial `.part` file if present. `on_progress`
/// gets `(downloaded, total)` byte counts. Verifies nothing against an upstream
/// hash (none is published) but records the file's own SHA-256 and rejects a
/// size mismatch on re-download.
pub fn download(
    app_db: &Connection,
    data_dir: &Path,
    name: &str,
    cancelled: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> AppResult<()> {
    let cat = entry(name)?;
    let check = disk_check(data_dir, name)?;
    if !check.ok {
        return Err(AppError::new(
            "WHISPER_DISK_FULL",
            "error.whisper.diskFull",
            format!(
                "need ~{} MB, have {} MB",
                check.needed_bytes / 1_048_576,
                check.free_bytes / 1_048_576
            ),
        )
        .with_details(
            serde_json::json!({ "needed": check.needed_bytes, "free": check.free_bytes }),
        ));
    }

    let root = models_root(data_dir);
    std::fs::create_dir_all(&root)?;
    let final_path = root.join(cat.file);
    let part_path = root.join(format!("{}.part", cat.file));

    let mut have = std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .map_err(|e| AppError::internal(format!("http: {e}")))?;
    let mut req = client.get(cat.url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let mut resp = req.send().map_err(|e| {
        AppError::new(
            "WHISPER_DOWNLOAD_FAILED",
            "error.whisper.downloadFailed",
            e.to_string(),
        )
    })?;
    let status = resp.status();
    if !status.is_success() {
        // A 416 means our `.part` is already the whole file.
        if status.as_u16() == 416 && have > 0 {
            std::fs::rename(&part_path, &final_path)?;
            return finish(app_db, name, &final_path);
        }
        return Err(AppError::new(
            "WHISPER_DOWNLOAD_FAILED",
            "error.whisper.downloadFailed",
            format!("HTTP {status}"),
        ));
    }
    // A 200 (not 206) means the server ignored Range — start over.
    if status.as_u16() == 200 && have > 0 {
        have = 0;
        let _ = std::fs::remove_file(&part_path);
    }
    let total = resp.content_length().unwrap_or(0) + have;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part_path)?;
    let mut buf = [0u8; 64 * 1024];
    let mut done = have;
    loop {
        if cancelled() {
            return Err(AppError::new(
                "WHISPER_DOWNLOAD_CANCELLED",
                "error.whisper.cancelled",
                "cancelled",
            ));
        }
        let n = resp.read(&mut buf).map_err(|e| {
            AppError::new(
                "WHISPER_DOWNLOAD_FAILED",
                "error.whisper.downloadFailed",
                e.to_string(),
            )
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        on_progress(done, total.max(done));
    }
    file.flush()?;
    drop(file);

    std::fs::rename(&part_path, &final_path)?;
    finish(app_db, name, &final_path)
}

fn finish(app_db: &Connection, name: &str, path: &Path) -> AppResult<()> {
    let mut hasher = Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = [0u8; 128 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let sha = format!("{:x}", hasher.finalize());
    let size = std::fs::metadata(path)?.len();

    // Reject a re-download whose hash changed against what we recorded before.
    let prev: Option<String> = app_db
        .query_row(
            "SELECT sha256 FROM whisper_models WHERE name = ?1",
            [name],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    if let Some(prev) = prev {
        if !prev.is_empty() && prev != sha {
            let _ = std::fs::remove_file(path);
            return Err(AppError::new(
                "WHISPER_SHA_MISMATCH",
                "error.whisper.shaMismatch",
                "downloaded file hash does not match the previously recorded one",
            ));
        }
    }

    app_db.execute(
        "INSERT INTO whisper_models (name, file_name, size_bytes, sha256, downloaded_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(name) DO UPDATE SET
           file_name = excluded.file_name, size_bytes = excluded.size_bytes,
           sha256 = excluded.sha256, downloaded_at = excluded.downloaded_at",
        params![
            name,
            path.to_string_lossy(),
            size as i64,
            sha,
            now_iso8601()
        ],
    )?;
    Ok(())
}

pub fn delete(app_db: &Connection, data_dir: &Path, name: &str) -> AppResult<()> {
    let path = model_path(data_dir, name)?;
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(models_root(data_dir).join(format!("{}.part", entry(name)?.file)));
    app_db.execute("DELETE FROM whisper_models WHERE name = ?1", [name])?;
    Ok(())
}

// ───────────────────────── transcription entry ─────────────────────────

/// Global serialisation: whisper is memory/GPU heavy (AC-5-10).
static TRANSCRIBE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct Transcript {
    pub segments: Vec<TranscriptSegment>,
    pub mostly_silent: bool,
}

/// Decode → VAD → per-region whisper → merge. `media` is any audio file or a
/// video container (its audio track is used). Blocking; call from a job thread.
pub fn transcribe_media(
    app_db: &Connection,
    data_dir: &Path,
    media: &Path,
    model_name: &str,
    language: Option<&str>,
) -> AppResult<Transcript> {
    let _guard = TRANSCRIBE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let model = model_path(data_dir, model_name)?;
    if !model.is_file() {
        return Err(AppError::new(
            "WHISPER_MODEL_MISSING",
            "error.whisper.modelMissing",
            format!("model '{model_name}' is not downloaded — get it in Settings"),
        ));
    }
    let _ = app_db; // model presence is filesystem-authoritative

    let pcm = audio::decode_to_mono_16k(media)?;
    let (regions, mostly_silent) = audio::speech_regions(&pcm, audio::TARGET_SR);

    let mut raw: Vec<TranscriptSegment> = Vec::new();
    for (s, e) in regions {
        let offset = s as f64 / audio::TARGET_SR as f64;
        let chunk_segs = engine::transcribe_chunk(&model, &pcm[s..e], language)?;
        for mut seg in chunk_segs {
            seg.start += offset;
            seg.end += offset;
            raw.push(seg);
        }
    }
    raw.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(Transcript {
        segments: audio::merge_segments(raw),
        mostly_silent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_the_four_settings_values() {
        for n in ["base", "small", "medium", "large-v2"] {
            assert!(entry(n).is_ok(), "missing catalog entry {n}");
        }
        assert!(entry("nope").is_err());
    }

    #[test]
    fn disk_check_reports_headroom() {
        let tmp = tempfile::tempdir().unwrap();
        let c = disk_check(tmp.path(), "base").unwrap();
        assert!(c.needed_bytes > entry("base").unwrap().approx_bytes);
    }
}
