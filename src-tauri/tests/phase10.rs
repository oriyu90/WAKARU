//! Phase 10 integration (docs/09 AC-10-*). Export -> import round-trip and the
//! version-policy gate.

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use wakaru_lib::domain::export::ExportInput;
use wakaru_lib::domain::project::CreateProjectInput;
use wakaru_lib::services::ingest::{self, IngestCtx, IngestInput};
use wakaru_lib::services::{export, projects, sources};
use wakaru_lib::storage;
use wakaru_lib::SourceKind;

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    app_db: Connection,
    app_db_path: PathBuf,
}
impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("projects");
        std::fs::create_dir_all(&root).unwrap();
        let app_db_path = tmp.path().join("app.db");
        let app_db = storage::open_app_db(&app_db_path).unwrap();
        Env { _tmp: tmp, root, app_db, app_db_path }
    }
}

fn seed_project(env: &Env, name: &str, body: &str) -> String {
    let pid = projects::create(
        &env.app_db,
        &env.root,
        CreateProjectInput { name: name.into(), description: Some("d".into()), color: None },
    )
    .unwrap()
    .id;
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("note.md");
    std::fs::write(&f, body).unwrap();
    let pdb = projects::open_db(&env.root, &pid).unwrap();
    let added = sources::add_one(&pdb, &env.root, &pid, &f).unwrap();
    let ctx = IngestCtx {
        project_db: &pdb,
        app_db: &env.app_db,
        project_id: &pid,
        source_id: &added.source.id,
        source_name: "note.md",
        project_dir: &projects::project_dir(&env.root, &pid),
    };
    ingest::run(&ctx, SourceKind::Markdown, &IngestInput::File(added.dest)).unwrap();
    pdb.execute("UPDATE sources SET status='ready' WHERE id=?1", [&added.source.id]).unwrap();
    pid
}

fn counts(root: &Path, pid: &str) -> (i64, i64, i64) {
    let db = projects::open_db(root, pid).unwrap();
    (
        db.query_row("SELECT count(*) FROM sources", [], |r| r.get(0)).unwrap(),
        db.query_row("SELECT count(*) FROM documents", [], |r| r.get(0)).unwrap(),
        db.query_row("SELECT count(*) FROM chunks", [], |r| r.get(0)).unwrap(),
    )
}

#[test]
fn ac_10_1_export_import_restores_sources_documents_and_chunks() {
    let env = Env::new();
    let pid = seed_project(&env, "量子計算の講義", "# 章1\n\n量子ビットの重ね合わせについて。\n\n# 章2\n\n干渉。");
    let before = counts(&env.root, &pid);
    assert!(before.0 == 1 && before.1 >= 2 && before.2 >= 2);

    let dest = env._tmp.path().join("exports");
    let res = export::export(
        &env.app_db,
        &env.root,
        &ExportInput { ids: vec![pid.clone()], dest_dir: dest.to_string_lossy().into(), include_embeddings: false },
    )
    .unwrap();
    assert_eq!(res.files.len(), 1);
    assert!(res.files[0].ends_with(".wakaru.zip"));

    // Import into the SAME app (must not collide) — a brand new project id.
    let imported = export::import(&env.app_db, &env.root, &res.files[0]).unwrap();
    assert_ne!(imported.id, pid);
    assert_eq!(imported.name, "量子計算の講義");

    let after = counts(&env.root, &imported.id);
    assert_eq!(after, before, "counts differ after round-trip");

    // Both projects now searchable in the cross-project mirror.
    let n: i64 = env
        .app_db
        .query_row("SELECT count(DISTINCT project_id) FROM global_index", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
    let _ = &env.app_db_path;
}

#[test]
fn ac_10_3_a_future_major_schema_is_rejected() {
    let env = Env::new();
    let dest = env._tmp.path().join("x");
    std::fs::create_dir_all(&dest).unwrap();
    // Hand-build a minimal zip with a too-new schemaVersion.
    let zip_path = dest.join("future.wakaru.zip");
    {
        let f = std::fs::File::create(&zip_path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        use zip::write::SimpleFileOptions;
        use std::io::Write;
        z.start_file("manifest.json", SimpleFileOptions::default()).unwrap();
        z.write_all(br#"{"schemaVersion":"9.0.0","project":{"name":"X"}}"#).unwrap();
        z.finish().unwrap();
    }
    let err = export::import(&env.app_db, &env.root, &zip_path.to_string_lossy()).unwrap_err();
    assert_eq!(err.code, "IMPORT_TOO_NEW");
}
