//! Phase 1 integration (docs/09 §3, AC-1-*). Exercises project + ingest + FTS
//! directly against the lib's public services, without the Tauri shell.

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use wakaru_lib::domain::project::CreateProjectInput;
use wakaru_lib::services::ingest::{self, IngestCtx, IngestInput};
use wakaru_lib::services::{projects, sources};
use wakaru_lib::storage;
use wakaru_lib::SourceKind;

struct Env {
    _tmp: tempfile::TempDir,
    projects_dir: PathBuf,
    app_db: Connection,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();
        let app_db = storage::open_app_db(&tmp.path().join("app.db")).unwrap();
        Env {
            _tmp: tmp,
            projects_dir,
            app_db,
        }
    }

    fn create_project(&self, name: &str) -> String {
        projects::create(
            &self.app_db,
            &self.projects_dir,
            CreateProjectInput {
                name: name.into(),
                description: None,
                color: None,
            },
        )
        .unwrap()
        .id
    }

    fn pdb(&self, project_id: &str) -> Connection {
        projects::open_db(&self.projects_dir, project_id).unwrap()
    }

    /// Add + run ingest synchronously; returns (source_id, docs, chunks).
    fn add_and_ingest(
        &self,
        project_id: &str,
        src: &Path,
        kind_hint: SourceKind,
    ) -> Result<(String, u32, u32), String> {
        let pdb = self.pdb(project_id);
        let added = sources::add_one(&pdb, &self.projects_dir, project_id, src)
            .map_err(|e| e.code.clone())?;
        let name: String = pdb
            .query_row(
                "SELECT original_name FROM sources WHERE id=?1",
                [&added.source.id],
                |r| r.get(0),
            )
            .unwrap();
        let ctx = IngestCtx {
            project_db: &pdb,
            app_db: &self.app_db,
            project_id,
            source_id: &added.source.id,
            source_name: &name,
            project_dir: &projects::project_dir(&self.projects_dir, project_id),
        };
        let out = ingest::run(
            &ctx,
            added.kind.unwrap_or(kind_hint),
            &IngestInput::File(added.dest.clone()),
        )
        .map_err(|e| e.code.clone())?;
        pdb.execute(
            "UPDATE sources SET status=?2, page_count=?3, analyzed_at='now' WHERE id=?1",
            rusqlite::params![
                added.source.id,
                if out.partial {
                    "ready_partial"
                } else {
                    "ready"
                },
                out.page_count.map(|v| v as i64),
            ],
        )
        .unwrap();
        Ok((added.source.id, out.documents, out.chunks))
    }
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn ac_1_1_create_makes_a_self_contained_folder() {
    let env = Env::new();
    let id = env.create_project("量子計算の講義");
    let dir = env.projects_dir.join(&id);
    assert!(dir.join("manifest.json").is_file());
    assert!(dir.join("project.db").is_file());
    for sub in ["sources", "derived", "workspace", "thumbs", "exports"] {
        assert!(dir.join(sub).is_dir(), "missing {sub}");
    }
}

#[test]
fn ac_1_3_text_family_produces_documents_and_chunks() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let s = tmp.path();

    let cases = [
        (
            write(
                s,
                "notes.md",
                "# 概要\n\n量子ビットは重ね合わせ状態を取る。\n\n# 詳細\n\nこれは説明です。",
            ),
            SourceKind::Markdown,
        ),
        (
            write(s, "data.json", r#"[{"k":1},{"k":2},{"k":3}]"#),
            SourceKind::Json,
        ),
        (
            write(s, "t.csv", "name,age\nAlice,30\nBob,25\n"),
            SourceKind::Sheet,
        ),
        (
            write(s, "log.txt", "line one\n\nline two paragraph\n\nline three"),
            SourceKind::Text,
        ),
    ];
    for (path, kind) in cases {
        let (_, docs, chunks) = env.add_and_ingest(&id, &path, kind).unwrap();
        assert!(
            docs >= 1 && chunks >= 1,
            "{kind:?}: docs={docs} chunks={chunks}"
        );
    }

    let pdb = env.pdb(&id);
    let fts: i64 = pdb
        .query_row("SELECT count(*) FROM chunks_fts", [], |r| r.get(0))
        .unwrap();
    assert!(fts >= 4, "fts rows = {fts}");
}

#[test]
fn ac_1_7_japanese_keyword_search_hits() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let md = write(
        tmp.path(),
        "q.md",
        "# 章1\n\n量子計算は重ね合わせと干渉を利用する計算方式である。",
    );
    env.add_and_ingest(&id, &md, SourceKind::Markdown).unwrap();

    let pdb = env.pdb(&id);
    let hits = wakaru_lib::keyword_search(&pdb, "量子", 10).unwrap();
    assert!(!hits.is_empty(), "expected a hit for 量子");
    assert!(hits[0].snippet.contains("量子"));

    let none = wakaru_lib::keyword_search(&pdb, "犬猫", 10).unwrap();
    assert!(none.is_empty(), "unexpected hit for 犬猫");
}

#[test]
fn ac_1_4_duplicate_sha_is_detected() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let a = write(tmp.path(), "a.txt", "identical bytes here");
    let b = write(tmp.path(), "b.txt", "identical bytes here");
    let pdb = env.pdb(&id);

    sources::add_one(&pdb, &env.projects_dir, &id, &a).expect("first ok");
    let err = sources::add_one(&pdb, &env.projects_dir, &id, &b).expect_err("second fails");
    assert_eq!(err.code, "SOURCE_DUPLICATE");
}

#[test]
fn ac_1_5_unsupported_format_is_kept_and_marked_failed() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let bin = write(tmp.path(), "zero.xyz", "not a known format at all");
    let pdb = env.pdb(&id);

    let added = sources::add_one(&pdb, &env.projects_dir, &id, &bin).expect("row still created");
    assert!(added.kind.is_none());
    let (status, code): (String, Option<String>) = pdb
        .query_row(
            "SELECT status, error_code FROM sources WHERE id=?1",
            [&added.source.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(code.as_deref(), Some("SOURCE_UNSUPPORTED_FORMAT"));
}

#[test]
fn ac_1_6_broken_json_fails_without_panic() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let broken = write(tmp.path(), "broken.json", "{ this : is not , valid json ,,");
    let res = env.add_and_ingest(&id, &broken, SourceKind::Json);
    assert_eq!(res.unwrap_err(), "SOURCE_PARSE");
}

#[test]
fn ac_1_10_delete_removes_documents_chunks_and_files() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let md = write(
        tmp.path(),
        "x.md",
        "# h\n\nsome body text here, long enough to chunk once.",
    );
    let (sid, _, _) = env.add_and_ingest(&id, &md, SourceKind::Markdown).unwrap();

    sources::delete(&env.app_db, &env.projects_dir, &id, &sid).unwrap();

    let pdb = env.pdb(&id);
    let docs: i64 = pdb
        .query_row(
            "SELECT count(*) FROM documents WHERE source_id=?1",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    let chunks: i64 = pdb
        .query_row(
            "SELECT count(*) FROM chunks WHERE source_id=?1",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!((docs, chunks), (0, 0));
    assert!(!env
        .projects_dir
        .join(&id)
        .join("derived")
        .join(&sid)
        .exists());
}

#[test]
fn image_ingest_normalises_and_writes_a_derived_page() {
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("photo.png");
    // 4000px wide -> must be resized to <= 2048.
    let buf = image::RgbImage::from_fn(4000, 100, |x, _| image::Rgb([(x % 256) as u8, 0, 0]));
    buf.save(&p).unwrap();

    let (sid, docs, chunks) = env.add_and_ingest(&id, &p, SourceKind::Image).unwrap();
    assert_eq!(docs, 1);
    assert!(chunks >= 1);
    let derived = env
        .projects_dir
        .join(&id)
        .join("derived")
        .join(&sid)
        .join("pages")
        .join("0001.png");
    assert!(derived.is_file(), "normalised image not written");
    let (w, _) = image::image_dimensions(&derived).unwrap();
    assert!(w <= 2048, "image not resized: {w}px");

    let pdb = env.pdb(&id);
    let (status, image_rel): (String, Option<String>) = pdb
        .query_row(
            "SELECT s.status, d.image_rel FROM sources s
             JOIN documents d ON d.source_id = s.id WHERE s.id = ?1",
            [&sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    // No Vision yet -> ready_partial, and the document points at the derived page.
    assert_eq!(status, "ready_partial");
    assert_eq!(image_rel.as_deref(), Some("pages/0001.png"));
}

#[test]
fn ac_1_11_ingest_and_search_work_offline() {
    // Nothing in the Phase-1 path touches the network — this test simply is the
    // proof that the whole add → ingest → search chain has no I/O beyond disk.
    let env = Env::new();
    let id = env.create_project("p");
    let tmp = tempfile::tempdir().unwrap();
    let md = write(
        tmp.path(),
        "n.md",
        "# t\n\nprice and value are synonyms in this note.",
    );
    env.add_and_ingest(&id, &md, SourceKind::Markdown).unwrap();
    let hits = wakaru_lib::keyword_search(&env.pdb(&id), "value", 5).unwrap();
    assert!(!hits.is_empty());
}
