//! Pure audio DSP for transcription (docs/04 §2-1/2-2). Decode any container
//! Symphonia understands to 16 kHz mono f32 (whisper.cpp's required format),
//! a light energy-gate VAD in place of Silero (DECISIONS D-15), 30-second
//! splitting at silence valleys, and readable segment merging.

use crate::domain::transcription::TranscriptSegment;
use crate::error::{AppError, AppResult};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const TARGET_SR: u32 = 16_000;
/// whisper works on 30 s windows; keep VAD chunks a touch under that.
const MAX_CHUNK_SEC: f32 = 28.0;
const FRAME_MS: usize = 20;

/// Decode `path` (audio file, or a video container's audio track) to mono f32 at
/// [`TARGET_SR`].
pub fn decode_to_mono_16k(path: &Path) -> AppResult<Vec<f32>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AppError::new("AV_DECODE_FAILED", "error.av.decodeFailed", e.to_string()))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.sample_rate.is_some())
        .ok_or_else(|| AppError::new("AV_NO_AUDIO", "error.av.noAudio", "no decodable audio track"))?
        .clone();
    let track_id = track.id;
    let src_sr = track.codec_params.sample_rate.unwrap_or(TARGET_SR);
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1).max(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AppError::new("AV_DECODE_FAILED", "error.av.decodeFailed", e.to_string()))?;

    let mut mono: Vec<f32> = Vec::new();
    let mut sbuf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(AppError::new("AV_DECODE_FAILED", "error.av.decodeFailed", e.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                if sbuf.is_none() {
                    sbuf = Some(SampleBuffer::new(decoded.capacity() as u64, *decoded.spec()));
                }
                let sb = sbuf.as_mut().unwrap();
                sb.copy_interleaved_ref(decoded);
                for frame in sb.samples().chunks(channels) {
                    let sum: f32 = frame.iter().copied().sum();
                    mono.push(sum / channels as f32);
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(AppError::new("AV_DECODE_FAILED", "error.av.decodeFailed", e.to_string())),
        }
    }

    if mono.is_empty() {
        return Err(AppError::new("AV_NO_AUDIO", "error.av.noAudio", "audio track decoded to nothing"));
    }
    Ok(resample_to_16k(&mono, src_sr))
}

/// Linear resample. Good enough for speech recognition input; avoids pulling a
/// heavier resampler config for the common 44.1/48 kHz → 16 kHz case.
pub fn resample_to_16k(input: &[f32], src_sr: u32) -> Vec<f32> {
    if src_sr == TARGET_SR || input.is_empty() {
        return input.to_vec();
    }
    let ratio = TARGET_SR as f64 / src_sr as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Per-frame RMS over `FRAME_MS` windows.
fn frame_rms(pcm: &[f32], sr: u32) -> Vec<f32> {
    let win = (sr as usize * FRAME_MS / 1000).max(1);
    pcm.chunks(win)
        .map(|c| {
            let s: f32 = c.iter().map(|x| x * x).sum();
            (s / c.len() as f32).sqrt()
        })
        .collect()
}

/// Speech regions as `(start_sample, end_sample)`, plus a flag when the file is
/// almost entirely silent (AC-5-4). Long regions are split under
/// [`MAX_CHUNK_SEC`] at their quietest interior frame (docs/04 §2-2).
pub fn speech_regions(pcm: &[f32], sr: u32) -> (Vec<(usize, usize)>, bool) {
    let rms = frame_rms(pcm, sr);
    if rms.is_empty() {
        return (Vec::new(), true);
    }
    let win = (sr as usize * FRAME_MS / 1000).max(1);
    let mut sorted = rms.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Noise floor from a low percentile — the median lands inside speech when
    // most of the file is speech, which would blow the threshold up.
    let floor = sorted[sorted.len() / 5];
    let peak = *sorted.last().unwrap();
    let thresh = (floor * 3.0).max(peak * 0.06).max(0.004);

    let voiced: Vec<bool> = rms.iter().map(|&r| r > thresh).collect();
    let voiced_frames = voiced.iter().filter(|v| **v).count();
    let mostly_silent = voiced_frames < 25 || (voiced_frames as f32 / voiced.len() as f32) < 0.02;

    // Group voiced frames, bridging gaps up to 400 ms.
    let bridge = 400 / FRAME_MS;
    let mut regions: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < voiced.len() {
        if !voiced[i] {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        let mut gap = 0;
        while i < voiced.len() {
            if voiced[i] {
                end = i;
                gap = 0;
            } else {
                gap += 1;
                if gap > bridge {
                    break;
                }
            }
            i += 1;
        }
        // Pad 100 ms each side, clamp to the buffer.
        let pad = 100 / FRAME_MS;
        let s = start.saturating_sub(pad) * win;
        let e = ((end + 1 + pad) * win).min(pcm.len());
        if e.saturating_sub(s) >= win * 10 {
            regions.push((s, e));
        }
    }

    let mut out = Vec::new();
    for r in regions {
        split_long(pcm, sr, r, &mut out);
    }
    if out.is_empty() && !pcm.is_empty() && !mostly_silent {
        out.push((0, pcm.len()));
    }
    (out, mostly_silent)
}

fn split_long(pcm: &[f32], sr: u32, region: (usize, usize), out: &mut Vec<(usize, usize)>) {
    let (s, e) = region;
    let max_samples = (MAX_CHUNK_SEC * sr as f32) as usize;
    if e - s <= max_samples {
        out.push(region);
        return;
    }
    // Quietest 20 ms frame in the middle third.
    let win = (sr as usize * FRAME_MS / 1000).max(1);
    let lo = s + (e - s) / 3;
    let hi = s + 2 * (e - s) / 3;
    let mut best = (s + e) / 2;
    let mut best_rms = f32::MAX;
    let mut p = lo;
    while p + win < hi {
        let frame = &pcm[p..p + win];
        let r: f32 = frame.iter().map(|x| x * x).sum::<f32>() / win as f32;
        if r < best_rms {
            best_rms = r;
            best = p;
        }
        p += win;
    }
    split_long(pcm, sr, (s, best), out);
    split_long(pcm, sr, (best, e), out);
}

/// Merge whisper's often-choppy output into readable lines: keep joining while
/// the running text is short and the previous piece didn't end a sentence.
pub fn merge_segments(segs: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    let mut out: Vec<TranscriptSegment> = Vec::new();
    for s in segs {
        let text = s.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(prev)
                if prev.text.chars().count() < 140
                    && !ends_sentence(&prev.text)
                    && s.start - prev.end < 1.5 =>
            {
                if !prev.text.ends_with(|c: char| c.is_whitespace()) {
                    prev.text.push(' ');
                }
                prev.text.push_str(&text);
                prev.end = s.end;
            }
            _ => out.push(TranscriptSegment { start: s.start, end: s.end, text }),
        }
    }
    out
}

fn ends_sentence(t: &str) -> bool {
    matches!(
        t.trim_end().chars().last(),
        Some('.' | '。' | '!' | '！' | '?' | '？' | '…')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_halves_length_roughly_for_32k() {
        let input = vec![0.5f32; 32_000];
        let out = resample_to_16k(&input, 32_000);
        assert!((out.len() as i64 - 16_000).abs() <= 1);
    }

    #[test]
    fn silence_is_flagged_and_yields_no_regions() {
        let pcm = vec![0.0f32; 16_000 * 3];
        let (regions, silent) = speech_regions(&pcm, 16_000);
        assert!(silent);
        assert!(regions.is_empty());
    }

    #[test]
    fn a_loud_burst_becomes_one_region() {
        let mut pcm = vec![0.0f32; 16_000 * 4];
        for (i, s) in pcm.iter_mut().enumerate() {
            if (16_000..48_000).contains(&i) {
                *s = if i % 2 == 0 { 0.4 } else { -0.4 };
            }
        }
        let (regions, silent) = speech_regions(&pcm, 16_000);
        assert!(!silent);
        assert_eq!(regions.len(), 1);
        let (s, e) = regions[0];
        assert!(s <= 16_000 && e >= 48_000);
    }

    #[test]
    fn long_region_is_split_under_the_window() {
        let pcm = vec![0.1f32; 16_000 * 90];
        let mut out = Vec::new();
        split_long(&pcm, 16_000, (0, pcm.len()), &mut out);
        assert!(out.len() >= 4);
        for (s, e) in out {
            assert!((e - s) as f32 / 16_000.0 <= MAX_CHUNK_SEC + 0.01);
        }
    }

    #[test]
    fn merge_joins_fragments_until_a_sentence_ends() {
        let seg = |a: f64, b: f64, t: &str| TranscriptSegment { start: a, end: b, text: t.into() };
        let merged = merge_segments(vec![
            seg(0.0, 1.0, "hello there"),
            seg(1.1, 2.0, "friend."),
            seg(2.2, 3.0, "next line"),
        ]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].text, "hello there friend.");
        assert_eq!(merged[0].end, 2.0);
    }
}
