//! Phase 3 integration (docs/09 AC-3-*). The AI-endpoint paths are covered by
//! the frontend + manual checks (docs/09 §4 says don't hit real APIs in tests);
//! here we lock down the pure retrieval + fusion logic and the FTS-only fallback.

use std::path::{Path, PathBuf};
use wakaru_lib::domain::project::CreateProjectInput;
use wakaru_lib::domain::search::{SearchQuery, SearchScope};
use wakaru_lib::services::ingest::{self, IngestCtx, IngestInput};
use wakaru_lib::services::{ai, projects, retrieval, search, sources};
use wakaru_lib::storage;
use wakaru_lib::SourceKind;

fn ingest_md(app_db: &rusqlite::Connection, root: &Path, pid: &str, name: &str, body: &str) {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join(name);
    std::fs::write(&p, body).unwrap();
    let pdb = projects::open_db(root, pid).unwrap();
    let added = sources::add_one(&pdb, root, pid, &p).unwrap();
    let ctx = IngestCtx {
        project_db: &pdb,
        app_db,
        project_id: pid,
        source_id: &added.source.id,
        source_name: name,
        project_dir: &projects::project_dir(root, pid),
    };
    ingest::run(&ctx, SourceKind::Markdown, &IngestInput::File(added.dest)).unwrap();
}

#[test]
fn rrf_fuses_two_rankings_by_reciprocal_rank() {
    let a = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let b = vec!["y".to_string(), "w".to_string(), "x".to_string()];
    let fused = retrieval::rrf(&[a, b]);
    // "y" is rank 2 then rank 1 -> highest combined score.
    assert_eq!(fused[0].0, "y");
    assert!(fused.iter().any(|(id, _)| id == "w"));
}

#[test]
fn e5_prefix_only_applies_to_e5_models() {
    assert_eq!(ai::e5_prefix("multilingual-e5-small", true), "query: ");
    assert_eq!(ai::e5_prefix("multilingual-e5-small", false), "passage: ");
    assert_eq!(ai::e5_prefix("text-embedding-3-small", true), "");
}

#[test]
fn ac_3_fts_only_hybrid_search_still_returns_hits_without_vectors() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    let app_db = storage::open_app_db(&tmp.path().join("app.db")).unwrap();
    let pid = projects::create(
        &app_db,
        &root,
        CreateProjectInput {
            name: "p".into(),
            description: None,
            color: None,
        },
    )
    .unwrap()
    .id;
    ingest_md(
        &app_db,
        &root,
        &pid,
        "q.md",
        "# 章1\n\n量子計算は重ね合わせを利用する。価格と価値についても触れる。",
    );

    let pdb = projects::open_db(&root, &pid).unwrap();
    // No chunk_vectors table exists -> query_vec None -> FTS path only.
    let hits = retrieval::hybrid_search(&pdb, "量子", None, None, None, 10).unwrap();
    assert!(!hits.is_empty());
    assert!(hits[0].snippet.contains("量子"));
}

#[test]
fn live_page_scope_filters_retrieval_to_the_current_ordinal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    let app_db = storage::open_app_db(&tmp.path().join("app.db")).unwrap();
    let pid = projects::create(
        &app_db,
        &root,
        CreateProjectInput {
            name: "p".into(),
            description: None,
            color: None,
        },
    )
    .unwrap()
    .id;
    let pdb = projects::open_db(&root, &pid).unwrap();
    pdb.execute(
        "INSERT INTO sources (id,kind,original_name,rel_path,status,added_at)
         VALUES ('s','pdf','pages.pdf','sources/s/pages.pdf','ready','now')",
        [],
    )
    .unwrap();
    for page in 1..=2 {
        let doc = format!("d{page}");
        let chunk = format!("c{page}");
        let text = format!("shared topic on page {page}");
        pdb.execute(
            "INSERT INTO documents (id,source_id,ordinal,kind,text,locator)
             VALUES (?1,'s',?2,'page',?3,?4)",
            rusqlite::params![
                doc,
                page,
                text,
                format!("{{\"t\":\"page\",\"page\":{page}}}")
            ],
        )
        .unwrap();
        pdb.execute(
            "INSERT INTO chunks (id,source_id,document_id,ordinal,text,text_bigram,locator,created_at)
             VALUES (?1,'s',?2,0,?3,?4,?5,'now')",
            rusqlite::params![chunk, doc, text, retrieval::cjk_bigram("shared topic"), format!("{{\"t\":\"page\",\"page\":{page}}}")],
        )
        .unwrap();
    }

    let hits =
        retrieval::hybrid_search(&pdb, "shared topic", None, Some("s"), Some(2), 10).unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|hit| hit.ordinal == 2));
}

#[tokio::test]
async fn ac_3_search_query_global_scope_spans_projects_via_fts() {
    let tmp = tempfile::tempdir().unwrap();
    let root: PathBuf = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    let app_db_path = tmp.path().join("app.db");
    let app_db = storage::open_app_db(&app_db_path).unwrap();

    let p1 = projects::create(
        &app_db,
        &root,
        CreateProjectInput {
            name: "Alpha".into(),
            description: None,
            color: None,
        },
    )
    .unwrap()
    .id;
    let p2 = projects::create(
        &app_db,
        &root,
        CreateProjectInput {
            name: "Beta".into(),
            description: None,
            color: None,
        },
    )
    .unwrap()
    .id;
    ingest_md(
        &app_db,
        &root,
        &p1,
        "a.md",
        "# a\n\nquantum entanglement notes",
    );
    ingest_md(
        &app_db,
        &root,
        &p2,
        "b.md",
        "# b\n\nquantum computing lecture",
    );
    drop(app_db);

    let res = search::query(
        &app_db_path,
        &root,
        SearchQuery {
            scope: SearchScope::Global,
            project_id: None,
            q: "quantum".into(),
            source_id: None,
            limit: 30,
            mode: None,
        },
    )
    .await
    .unwrap();

    assert!(!res.semantic);
    let names: std::collections::HashSet<_> =
        res.hits.iter().map(|h| h.project_name.clone()).collect();
    assert!(
        names.contains("Alpha") && names.contains("Beta"),
        "{names:?}"
    );
}
