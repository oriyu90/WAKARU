//! The `whisper-rs` call. Blocking and CPU/GPU-heavy — only ever run from a
//! `spawn_blocking` job thread, serialised by the lock in `super`.

use crate::domain::transcription::TranscriptSegment;
use crate::error::{AppError, AppResult};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Transcribe one PCM chunk (16 kHz mono f32). Timestamps are relative to the
/// start of `samples`; the caller adds the chunk offset.
pub fn transcribe_chunk(
    model_path: &Path,
    samples: &[f32],
    language: Option<&str>,
) -> AppResult<Vec<TranscriptSegment>> {
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| {
            AppError::new(
                "WHISPER_LOAD_FAILED",
                "error.whisper.loadFailed",
                e.to_string(),
            )
        })?;
    let mut state = ctx.create_state().map_err(|e| {
        AppError::new(
            "WHISPER_STATE_FAILED",
            "error.whisper.stateFailed",
            e.to_string(),
        )
    })?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    params.set_n_threads((threads / 2).max(1) as i32);
    params.set_translate(false);
    if let Some(l) = language {
        params.set_language(Some(l));
    }
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);

    state.full(params, samples).map_err(|e| {
        AppError::new(
            "WHISPER_RUN_FAILED",
            "error.whisper.runFailed",
            e.to_string(),
        )
    })?;

    // whisper-rs 0.16 returns plain values for segment queries (bounds are
    // enforced by `get_segment`); failures collapse to skip, as before.
    let n = state.full_n_segments();
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let Some(seg) = state.get_segment(i) else {
            continue;
        };
        let text = seg.to_str_lossy().unwrap_or_default();
        let t0 = seg.start_timestamp();
        let t1 = seg.end_timestamp();
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        out.push(TranscriptSegment {
            // whisper timestamps are in centiseconds.
            start: t0 as f64 / 100.0,
            end: t1 as f64 / 100.0,
            text,
        });
    }
    Ok(out)
}
