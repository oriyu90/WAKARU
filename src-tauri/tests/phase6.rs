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
    let pid = projects::create(&app_db, &root, CreateProjectInput { name: "p".into(), description: None, color: None })
        .unwrap()
        .id;
    let db = projects::open_db(&root, &pid).unwrap();

    let a = studio::create_tab(&db, None).unwrap();
    let b = studio::create_tab(&db, Some("  Draft  ".into())).unwrap();
    assert_eq!(b.title, "Draft");
    assert_eq!(studio::list_tabs(&db).unwrap().len(), 2);

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
    let pid = projects::create(&app_db, &root, CreateProjectInput { name: "p".into(), description: None, color: None })
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
    assert!(ws.join("summary.md").is_file(), "workspace file must survive tab close");
    let arts = studio::list_artifacts(&db).unwrap();
    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0].thread_id, None);
}

#[test]
fn write_file_rejects_paths_outside_the_workspace() {
    let (_tmp, root, app_db) = env();
    let pid = projects::create(&app_db, &root, CreateProjectInput { name: "p".into(), description: None, color: None })
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
    assert!(!projects::project_dir(&root, &pid).join("escape.txt").exists());
}

#[test]
fn read_only_tools_report_project_state() {
    let (_tmp, root, app_db) = env();
    let pid = projects::create(&app_db, &root, CreateProjectInput { name: "p".into(), description: None, color: None })
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

    studio::dispatch_tool(&db, &ws, &tab.thread_id, "write_file", r#"{"path":"a.txt","content":"AAA"}"#).unwrap();
    let files = studio::dispatch_tool(&db, &ws, &tab.thread_id, "list_files", "{}").unwrap();
    assert!(files.contains("a.txt"));
    let body = studio::dispatch_tool(&db, &ws, &tab.thread_id, "read_file", r#"{"path":"a.txt"}"#).unwrap();
    assert_eq!(body, "AAA");
}

#[test]
fn read_tab_lets_one_conversation_read_another() {
    let (_tmp, root, app_db) = env();
    let pid = projects::create(&app_db, &root, CreateProjectInput { name: "p".into(), description: None, color: None })
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
    let dump = studio::dispatch_tool(&db, &ws, &here.thread_id, "read_tab", r#"{"title":"research"}"#).unwrap();
    assert!(dump.contains("the market doubled"));
}
