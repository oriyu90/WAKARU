//! Phase 5 integration (docs/09 AC-5-*). Real whisper runs need a downloaded
//! model and are exercised manually (docs/09 §4/§5); here we lock the decode
//! path, the "model not downloaded" guard, and the disk pre-check.

use std::io::Write;
use wakaru_lib::services::whisper;

/// A minimal PCM-16 mono WAV, `secs` of the given constant amplitude.
fn write_wav(path: &std::path::Path, sr: u32, secs: f32, amp: i16) {
    let n = (sr as f32 * secs) as u32;
    let data_len = n * 2;
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 2).to_le_bytes()).unwrap(); // byte rate
    f.write_all(&2u16.to_le_bytes()).unwrap(); // block align
    f.write_all(&16u16.to_le_bytes()).unwrap(); // bits
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for i in 0..n {
        let s = if amp != 0 && i % 2 == 0 {
            amp
        } else if amp != 0 {
            -amp
        } else {
            0
        };
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

#[test]
fn decodes_a_wav_and_flags_silence() {
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("silent.wav");
    write_wav(&wav, 16_000, 1.5, 0);

    let pcm = whisper::audio::decode_to_mono_16k(&wav).unwrap();
    assert!((pcm.len() as i64 - 24_000).abs() < 200, "≈1.5s at 16 kHz");
    let (regions, silent) = whisper::audio::speech_regions(&pcm, whisper::audio::TARGET_SR);
    assert!(silent, "a flat-zero WAV must read as silent (AC-5-4)");
    assert!(regions.is_empty());
}

#[test]
fn resamples_a_44k_wav_to_16k() {
    let tmp = tempfile::tempdir().unwrap();
    let wav = tmp.path().join("tone.wav");
    write_wav(&wav, 44_100, 1.0, 6000);
    let pcm = whisper::audio::decode_to_mono_16k(&wav).unwrap();
    assert!((pcm.len() as i64 - 16_000).abs() < 400);
}

#[test]
fn transcribe_without_a_downloaded_model_is_a_clean_error() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path();
    let db = wakaru_lib::storage::open_app_db(&data_dir.join("app.db")).unwrap();
    let wav = data_dir.join("a.wav");
    write_wav(&wav, 16_000, 0.5, 3000);

    let err = whisper::transcribe_media(&db, data_dir, &wav, "small", None).unwrap_err();
    assert_eq!(err.code, "WHISPER_MODEL_MISSING");
}

#[test]
fn disk_check_needs_more_than_the_raw_model_size() {
    let tmp = tempfile::tempdir().unwrap();
    let c = whisper::disk_check(tmp.path(), "small").unwrap();
    assert!(c.needed_bytes > 400 * 1024 * 1024);
    assert!(c.free_bytes > 0);
}
