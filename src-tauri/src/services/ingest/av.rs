//! Audio / video ingest (docs/04 §2, §6). Decode → VAD → whisper → readable
//! segments, one `Unit` per merged transcript line with a `time` locator
//! (AC-5-3/5-8/5-9). Video plays back via the WebView `<video>` + `wakaru-asset://`
//! and use embedded Silero VAD before transcription.

use super::Unit;
use crate::error::AppResult;
use crate::services::{settings, whisper};
use rusqlite::Connection;
use std::path::Path;

pub fn parse_av(app_db: &Connection, data_dir: &Path, media: &Path) -> AppResult<Vec<Unit>> {
    let s = settings::get(app_db)?;
    let model = s.transcription.whisper_model.clone();
    let lang = match s.transcription.language.as_str() {
        "auto" | "" => None,
        other => Some(other.to_string()),
    };

    let t = whisper::transcribe_media(app_db, data_dir, media, &model, lang.as_deref())?;

    let mut units = Vec::new();
    if t.mostly_silent {
        // AC-5-4 — surfaced as the first "segment" so the Viewer shows it.
        units.push(Unit {
            ordinal: 0,
            kind: "notice",
            title: Some("⚠︎".into()),
            text: "音声が検出されませんでした。ファイルに発話が含まれていない可能性があります。 / No speech detected.".into(),
            locator: serde_json::json!({ "t": "time", "start": 0.0, "end": 0.0 }),
        });
    }

    for (i, seg) in t.segments.iter().enumerate() {
        units.push(Unit {
            ordinal: (units.len()) as u32,
            kind: "segment",
            title: Some(fmt_ts(seg.start)),
            text: seg.text.clone(),
            locator: serde_json::json!({ "t": "time", "start": seg.start, "end": seg.end }),
        });
        let _ = i;
    }

    if units.is_empty() {
        units.push(Unit {
            ordinal: 0,
            kind: "notice",
            title: Some("⚠︎".into()),
            text: "文字起こし結果が空でした。 / The transcript was empty.".into(),
            locator: serde_json::json!({ "t": "time", "start": 0.0, "end": 0.0 }),
        });
    }
    Ok(units)
}

fn fmt_ts(sec: f64) -> String {
    let s = sec.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}
