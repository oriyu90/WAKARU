//! Phase 6 integration (docs/09 AC-6-*). The AI loop itself is exercised
//! manually (docs/09 §4/§5); here we lock the tab lifecycle, the workspace
//! path guard, the built-in tools, and the "closing a tab keeps its files"
//! guarantee (AC-6-10).

use rusqlite::params;
use wakaru_lib::domain::project::CreateProjectInput;
use wakaru_lib::services::{projects, studio};
use wakaru_lib::storage;

fn env() -> (tempfile::TempDir, std::path::PathBuf, rusqlite::Connection) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    let app_db = storage::open_app_db(&tmp.path().join("app.db")).unwrap();
    (tmp, root, app_db)
}

#[test]
fn ac_6_1_tabs_can_be_created_renamed_reordered_and_closed() {
    let (_tmp, root, app_db) = env();
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
    let db = projects::open_db(&root, &pid).unwrap();

    let a = studio::create_tab(&db, None).unwrap();
    let b = studio::create_tab(&db, Some("  Draft  ".into())).unwrap();
    assert_eq!(b.title, "Draft");
    assert_eq!(studio::list_tabs(&db).unwrap().len(), 2);

    // list_tabs is metadata-only; get_tab carries the conversation.
    let pdb = projects::open_db(&root, &pid).unwrap();
    let th: String = pdb
        .query_row(
            "SELECT thread_id FROM studio_tabs WHERE id = ?1",
            [&a.id],
            |r| r.get(0),
        )
        .unwrap();
    pdb.execute(
        "INSERT INTO messages (id, thread_id, role, content, created_at)
         VALUES ('mm1', ?1, 'user', 'hi', 'now')",
        rusqlite::params![th],
    )
    .unwrap();
    let listed = studio::list_tabs(&db).unwrap();
    let a_row = listed.iter().find(|t| t.id == a.id).unwrap();
    assert_eq!(a_row.message_count, 1);
    assert!(a_row.messages.is_empty(), "list_tabs never ships bodies");
    let full = studio::get_tab(&db, &a.id).unwrap();
    assert_eq!(full.messages.len(), 1);
    assert_eq!(full.messages[0].content, "hi");

    studio::rename_tab(&db, &a.id, "Renamed").unwrap();
    studio::reorder_tabs(&db, &[b.id.clone(), a.id.clone()]).unwrap();
    let tabs = studio::list_tabs(&db).unwrap();
    assert_eq!(tabs[0].id, b.id);
    assert_eq!(tabs[1].title, "Renamed");

    studio::close_tab(&db, &a.id).unwrap();
    let tabs = studio::list_tabs(&db).unwrap();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].id, b.id);
}

#[test]
fn ac_6_10_closing_a_tab_keeps_its_workspace_files_and_artifacts() {
    let (_tmp, root, app_db) = env();
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
    let db = projects::open_db(&root, &pid).unwrap();
    let ws = projects::project_dir(&root, &pid).join("workspace");

    let tab = studio::create_tab(&db, None).unwrap();
    let out = studio::dispatch_tool(
        &db,
        &ws,
        &tab.thread_id,
        "write_file",
        r#"{ "path": "summary.md", "content": "Notes hello" }"#,
    )
    .unwrap();
    assert!(out.contains("summary.md"));
    assert!(ws.join("summary.md").is_file());
    assert_eq!(studio::list_artifacts(&db).unwrap().len(), 1);

    studio::close_tab(&db, &tab.id).unwrap();

    // File on disk survives; artifact row survives with a null thread_id.
    assert!(
        ws.join("summary.md").is_file(),
        "workspace file must survive tab close"
    );
    let arts = studio::list_artifacts(&db).unwrap();
    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0].thread_id, None);
}

#[test]
fn write_file_rejects_paths_outside_the_workspace() {
    let (_tmp, root, app_db) = env();
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
    let db = projects::open_db(&root, &pid).unwrap();
    let ws = projects::project_dir(&root, &pid).join("workspace");
    let tab = studio::create_tab(&db, None).unwrap();

    let err = studio::dispatch_tool(
        &db,
        &ws,
        &tab.thread_id,
        "write_file",
        r#"{ "path": "../escape.txt", "content": "x" }"#,
    )
    .unwrap_err();
    assert_eq!(err.code, "SANDBOX_PATH_DENIED");
    assert!(!projects::project_dir(&root, &pid)
        .join("escape.txt")
        .exists());
}

#[test]
fn read_only_tools_report_project_state() {
    let (_tmp, root, app_db) = env();
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
    let db = projects::open_db(&root, &pid).unwrap();
    let ws = projects::project_dir(&root, &pid).join("workspace");
    db.execute(
        "INSERT INTO sources (id, kind, original_name, rel_path, status, added_at)
         VALUES ('s1','markdown','notes.md','sources/s1/notes.md','ready','now')",
        [],
    )
    .unwrap();
    let tab = studio::create_tab(&db, None).unwrap();

    let sources = studio::dispatch_tool(&db, &ws, &tab.thread_id, "list_sources", "{}").unwrap();
    assert!(sources.contains("notes.md"));

    studio::dispatch_tool(
        &db,
        &ws,
        &tab.thread_id,
        "write_file",
        r#"{"path":"a.txt","content":"AAA"}"#,
    )
    .unwrap();
    let files = studio::dispatch_tool(&db, &ws, &tab.thread_id, "list_files", "{}").unwrap();
    assert!(files.contains("a.txt"));
    let body = studio::dispatch_tool(&db, &ws, &tab.thread_id, "read_file", r#"{"path":"a.txt"}"#)
        .unwrap();
    assert_eq!(body, "AAA");
}

#[test]
fn read_tab_lets_one_conversation_read_another() {
    let (_tmp, root, app_db) = env();
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
    let db = projects::open_db(&root, &pid).unwrap();
    let ws = projects::project_dir(&root, &pid).join("workspace");

    let other = studio::create_tab(&db, Some("Research".into())).unwrap();
    db.execute(
        "INSERT INTO messages (id, thread_id, role, content, created_at)
         VALUES ('m1', ?1, 'user', 'what did we conclude?', 'now')",
        params![other.thread_id],
    )
    .unwrap();
    db.execute(
        "INSERT INTO messages (id, thread_id, role, content, created_at)
         VALUES ('m2', ?1, 'assistant', 'the market doubled', 'now')",
        params![other.thread_id],
    )
    .unwrap();

    let here = studio::create_tab(&db, None).unwrap();
    let dump = studio::dispatch_tool(
        &db,
        &ws,
        &here.thread_id,
        "read_tab",
        r#"{"title":"research"}"#,
    )
    .unwrap();
    assert!(dump.contains("the market doubled"));
}

#[test]
fn build_document_writes_a_docx_artifact_and_guards_the_path() {
    let (_tmp, root, app_db) = env();
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
    let db = projects::open_db(&root, &pid).unwrap();
    let ws = projects::project_dir(&root, &pid).join("workspace");
    let tab = studio::create_tab(&db, None).unwrap();

    let args = serde_json::json!({
        "path": "report.docx",
        "format": "docx",
        "title": "四半期レポート",
        "toc": true,
        "sections": [
            { "level": 1, "heading": "概要", "body": "本文です。**重要**。\n\n- 項目A\n- 項目B" },
            { "level": 2, "heading": "詳細", "body": "| 名前 | 値 |\n| - | - |\n| x | 1 |" }
        ]
    })
    .to_string();

    let out = studio::dispatch_tool(&db, &ws, &tab.thread_id, "build_document", &args).unwrap();
    assert!(out.contains("report.docx"), "{out}");

    let file = ws.join("report.docx");
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(&bytes[..2], b"PK", "docx must be a zip");
    assert_eq!(studio::list_artifacts(&db).unwrap().len(), 1);

    // path guard still applies to build_document
    let err = studio::dispatch_tool(
        &db,
        &ws,
        &tab.thread_id,
        "build_document",
        r#"{ "path": "../evil.md", "format": "md", "title": "T", "sections": [{ "body": "b" }] }"#,
    )
    .unwrap_err();
    assert_eq!(err.code, "SANDBOX_PATH_DENIED");

    // a pdf request produces a real PDF, or falls back to markdown when this
    // machine has no CJK-capable system font.
    let pdf_args = serde_json::json!({
        "path": "note.pdf", "format": "pdf", "title": "T",
        "sections": [{ "body": "hello" }]
    })
    .to_string();
    let out = studio::dispatch_tool(&db, &ws, &tab.thread_id, "build_document", &pdf_args).unwrap();
    let pdf = ws.join("note.pdf");
    let md = ws.join("note.md");
    assert!(
        pdf.is_file() || md.is_file(),
        "pdf request produced nothing: {out}"
    );
    if pdf.is_file() {
        assert_eq!(&std::fs::read(&pdf).unwrap()[..5], b"%PDF-");
    } else {
        assert!(out.contains("note.md"), "{out}");
    }
}
