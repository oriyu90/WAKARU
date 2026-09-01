//! Phase 8 integration (docs/09 AC-8-*). The organizer streaming path is a
//! manual check (needs a live model); here we lock the two file writers.

use wakaru_lib::domain::file_modifier::{ImagesToPdfInput, SaveTextInput};
use wakaru_lib::services::file_modifier as fm;

fn png(dir: &std::path::Path, name: &str, w: u32, h: u32) -> std::path::PathBuf {
    let p = dir.join(name);
    image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 255) as u8, (y % 255) as u8, 90])
    })
    .save(&p)
    .unwrap();
    p
}

#[test]
fn ac_8_1_multiple_images_become_one_pdf_with_chosen_options() {
    let tmp = tempfile::tempdir().unwrap();
    let a = png(tmp.path(), "a.png", 400, 300); // landscape
    let b = png(tmp.path(), "b.png", 300, 500); // portrait
    let dest = tmp.path().join("combined.pdf");

    let out = fm::images_to_pdf(&ImagesToPdfInput {
        images: vec![a.to_string_lossy().into(), b.to_string_lossy().into()],
        page_size: "a4".into(),
        orientation: "auto".into(),
        margin: "sm".into(),
        fit: "cover".into(),
        dest_path: dest.to_string_lossy().into(),
    })
    .unwrap();

    let bytes = std::fs::read(&out.path).unwrap();
    assert_eq!(&bytes[..4], b"%PDF");
    // two embedded images -> a non-trivial file
    assert!(bytes.len() > 5_000, "pdf too small: {}", bytes.len());
    // Parse the complete object graph, rather than accepting a file that only
    // happens to start with the PDF magic bytes.
    assert!(pdf_extract::extract_text(&out.path).is_ok());
}

#[test]
fn ac_8_no_images_is_a_clean_error_not_a_panic() {
    let err = fm::images_to_pdf(&ImagesToPdfInput {
        images: vec![],
        page_size: "a4".into(),
        orientation: "auto".into(),
        margin: "none".into(),
        fit: "contain".into(),
        dest_path: "/tmp/x.pdf".into(),
    })
    .unwrap_err();
    assert_eq!(err.code, "FM_NO_IMAGES");
}

#[test]
fn save_text_txt_and_md_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let md = fm::save_text(&SaveTextInput {
        content: "# H\n\n- one\n- two".into(),
        dest_path: tmp.path().join("out").to_string_lossy().into(),
        format: "md".into(),
    })
    .unwrap();
    assert!(md.path.ends_with("out.md"));

    let txt = fm::save_text(&SaveTextInput {
        content: "plain body".into(),
        dest_path: tmp.path().join("out.txt").to_string_lossy().into(),
        format: "txt".into(),
    })
    .unwrap();
    assert_eq!(std::fs::read_to_string(&txt.path).unwrap(), "plain body");
}
