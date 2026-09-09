//! Phase 2 integration (docs/09 AC-2-*). Viewer tab state + document access,
//! without the Tauri shell.

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use wakaru_lib::domain::project::CreateProjectInput;
use wakaru_lib::services::ingest::{self, IngestCtx, IngestInput};
use wakaru_lib::services::{assets, projects, sources, viewer};
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
    fn project(&self) -> String {
        projects::create(
            &self.app_db,
            &self.projects_dir,
            CreateProjectInput {
                name: "p".into(),
                description: None,
                color: None,
            },
        )
        .unwrap()
        .id
    }
    fn pdb(&self, id: &str) -> Connection {
        projects::open_db(&self.projects_dir, id).unwrap()
    }
    fn ingest_file(&self, pid: &str, path: &Path, kind: SourceKind) -> String {
        let pdb = self.pdb(pid);
        let added = sources::add_one(&pdb, &self.projects_dir, pid, path).unwrap();
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
            project_id: pid,
            source_id: &added.source.id,
            source_name: &name,
            project_dir: &projects::project_dir(&self.projects_dir, pid),
        };
        let out = ingest::run(
            &ctx,
            added.kind.unwrap_or(kind),
            &IngestInput::File(added.dest),
        )
        .unwrap();
        pdb.execute(
            "UPDATE sources SET status=?2, page_count=?3 WHERE id=?1",
            rusqlite::params![
                added.source.id,
                if out.partial {
                    "ready_partial"
                } else {
                    "ready"
                },
                out.page_count.map(|v| v as i64)
            ],
        )
        .unwrap();
        added.source.id
    }
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn website_folder_import_extracts_a_unit_per_html_and_lists_files() {
    use wakaru_lib::services::website;
    let env = Env::new();
    let pid = env.project();
    let pdb = env.pdb(&pid);

    // A tiny site on disk.
    let site = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(site.path().join("css")).unwrap();
    std::fs::write(
        site.path().join("index.html"),
        "<title>Home</title><h1>Home</h1><p>Welcome to the demo.</p>",
    )
    .unwrap();
    std::fs::write(
        site.path().join("about.html"),
        "<title>About</title><p>About the demo.</p>",
    )
    .unwrap();
    std::fs::write(site.path().join("css/site.css"), "body{color:#222}").unwrap();

    // Copy into sources/<id>/ the way `sources::add_folder` would.
    let sid = uuid::Uuid::now_v7().to_string();
    let dest = projects::project_dir(&env.projects_dir, &pid)
        .join("sources")
        .join(&sid);
    std::fs::create_dir_all(&dest).unwrap();
    let copied = website::copy_site_tree(site.path(), &dest).unwrap();
    assert_eq!(copied.entry, "index.html");
    assert_eq!(copied.file_count, 3);

    let rel_path = format!("sources/{sid}/{}", copied.entry);
    pdb.execute(
        "INSERT INTO sources (id, kind, original_name, rel_path, mime, bytes, status, added_at)
         VALUES (?1, 'website', 'demo', ?2, 'text/html', ?3, 'queued', ?4)",
        rusqlite::params![
            sid,
            rel_path,
            copied.total_bytes as i64,
            wakaru_lib::storage::migrate::now_iso8601()
        ],
    )
    .unwrap();

    let ctx = IngestCtx {
        project_db: &pdb,
        app_db: &env.app_db,
        project_id: &pid,
        source_id: &sid,
        source_name: "demo",
        project_dir: &projects::project_dir(&env.projects_dir, &pid),
    };
    let out = ingest::run(&ctx, SourceKind::Website, &IngestInput::File(dest.clone())).unwrap();
    assert_eq!(out.documents, 2, "one unit per HTML file");
    assert_eq!(out.page_count, Some(2));

    let first: String = pdb
        .query_row(
            "SELECT text FROM documents WHERE source_id=?1 ORDER BY ordinal LIMIT 1",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    assert!(first.contains("Welcome to the demo."));

    let manifest = website::manifest(&dest, &copied.entry).unwrap();
    assert_eq!(manifest.entry, "index.html");
    assert!(manifest.files.iter().any(|f| f.path == "css/site.css"));
    assert!(manifest
        .files
        .iter()
        .any(|f| f.path == "index.html" && f.is_entry));
}

#[test]
fn ac_2_3_opening_a_source_twice_reuses_one_tab() {
    let env = Env::new();
    let pid = env.project();
    let tmp = tempfile::tempdir().unwrap();
    let sid = env.ingest_file(
        &pid,
        &write(tmp.path(), "a.md", "# h\n\nbody text here"),
        SourceKind::Markdown,
    );
    let pdb = env.pdb(&pid);

    let t1 = viewer::open_tab(&pdb, &sid, None).unwrap();
    let t2 = viewer::open_tab(
        &pdb,
        &sid,
        Some(serde_json::json!({ "t": "page", "page": 2 })),
    )
    .unwrap();
    assert_eq!(t1.id, t2.id, "same source must reuse the tab");
    assert_eq!(viewer::get_tabs(&pdb).unwrap().len(), 1);
}

#[test]
fn ac_2_6_tabs_persist_across_reopen() {
    let env = Env::new();
    let pid = env.project();
    let tmp = tempfile::tempdir().unwrap();
    let sid = env.ingest_file(
        &pid,
        &write(tmp.path(), "b.txt", "hello world"),
        SourceKind::Text,
    );

    {
        let pdb = env.pdb(&pid);
        viewer::open_tab(&pdb, &sid, None).unwrap();
    }
    // A fresh connection = "reopening the project".
    let pdb2 = env.pdb(&pid);
    let tabs = viewer::get_tabs(&pdb2).unwrap();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].source_id, sid);
}

#[test]
fn get_document_returns_text_and_clamps_ordinal() {
    let env = Env::new();
    let pid = env.project();
    let tmp = tempfile::tempdir().unwrap();
    let sid = env.ingest_file(
        &pid,
        &write(
            tmp.path(),
            "c.md",
            "# 1\n\nalpha\n\n# 2\n\nbeta\n\n# 3\n\ngamma",
        ),
        SourceKind::Markdown,
    );
    let pdb = env.pdb(&pid);

    let d2 = viewer::get_document(&pdb, &env.projects_dir, &pid, &sid, 2).unwrap();
    assert_eq!(d2.ordinal, 2);
    assert!(d2.total >= 3);
    assert!(d2.text.contains("beta"));

    // out-of-range clamps, never errors
    let d99 = viewer::get_document(&pdb, &env.projects_dir, &pid, &sid, 99).unwrap();
    assert_eq!(d99.ordinal, d99.total);
}

#[test]
fn source_detail_exposes_asset_urls_for_image_and_pdf_page_count() {
    let env = Env::new();
    let pid = env.project();
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("pic.png");
    image::RgbImage::from_pixel(64, 64, image::Rgb([10, 20, 30]))
        .save(&p)
        .unwrap();
    let sid = env.ingest_file(&pid, &p, SourceKind::Image);
    let pdb = env.pdb(&pid);

    let detail = viewer::source_detail(&pdb, &pid, &sid).unwrap();
    assert_eq!(detail.kind, SourceKind::Image);
    let url = detail
        .primary_asset_url
        .expect("image should have an asset url");
    assert!(url.starts_with("wakaru-asset://localhost/"));
    assert!(url.contains(&sid));
}

#[test]
fn asset_resolver_only_serves_files_inside_the_project_sandbox() {
    let env = Env::new();
    let pid = env.project();
    let tmp = tempfile::tempdir().unwrap();
    let sid = env.ingest_file(
        &pid,
        &write(tmp.path(), "d.txt", "safe body"),
        SourceKind::Text,
    );

    // The copied original is reachable.
    let ok = assets::resolve(
        &env.projects_dir,
        &format!("/{pid}/{sid}/sources/{sid}/d.txt"),
    );
    assert!(ok.is_ok(), "{ok:?}");

    // Anything climbing out is refused.
    for bad in [
        format!("/{pid}/{sid}/../../../../etc/passwd"),
        format!("/{pid}/{sid}/sources/{sid}/../../../secret"),
    ] {
        assert!(
            assets::resolve(&env.projects_dir, &bad).is_err(),
            "should deny {bad}"
        );
    }
}
