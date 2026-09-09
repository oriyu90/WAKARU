//! Phase 4 integration (docs/09 AC-4-*). The streaming / AI paths are covered
//! manually (docs/09 §4/§5); here we lock the persistence + import contracts.

use rusqlite::params;
use wakaru_lib::domain::illustrator::ImportToStudioInput;
use wakaru_lib::domain::project::CreateProjectInput;
use wakaru_lib::services::{illustrator, projects};
use wakaru_lib::storage;

fn env() -> (tempfile::TempDir, std::path::PathBuf, rusqlite::Connection) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    let app_db = storage::open_app_db(&tmp.path().join("app.db")).unwrap();
    (tmp, root, app_db)
}

#[test]
fn ac_4_5_thread_and_messages_persist_across_reopen() {
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

    // A source + a document so the thread has something to attach to.
    {
        let pdb = projects::open_db(&root, &pid).unwrap();
        pdb.execute(
            "INSERT INTO sources (id, kind, original_name, rel_path, status, added_at)
             VALUES ('s1','markdown','n.md','sources/s1/n.md','ready','now')",
            [],
        )
        .unwrap();
    }

    let tid = {
        let pdb = projects::open_db(&root, &pid).unwrap();
        let th = illustrator::get_or_create_thread(&pdb, "s1", "page:3").unwrap();
        pdb.execute(
            "INSERT INTO messages (id, thread_id, role, content, created_at)
             VALUES ('m1', ?1, 'user', 'what is this?', 'now')",
            params![th.id],
        )
        .unwrap();
        pdb.execute(
            "INSERT INTO messages (id, thread_id, role, content, created_at)
             VALUES ('m2', ?1, 'assistant', 'it is a heading', 'now')",
            params![th.id],
        )
        .unwrap();
        th.id
    };

    // "Reopen": a fresh connection, same locator key -> same thread + messages.
    let pdb2 = projects::open_db(&root, &pid).unwrap();
    let again = illustrator::get_or_create_thread(&pdb2, "s1", "page:3").unwrap();
    assert_eq!(again.id, tid);
    assert_eq!(again.messages.len(), 2);
    assert_eq!(again.messages[0].content, "what is this?");

    // A different locator key -> a different thread.
    let other = illustrator::get_or_create_thread(&pdb2, "s1", "page:4").unwrap();
    assert_ne!(other.id, tid);
    assert!(other.messages.is_empty());

    // Viewer page navigation continues to request the canonical whole-source
    // key, so a source-scoped Live session and its explanation are not forked.
    let whole = illustrator::get_or_create_thread(&pdb2, "s1", "whole").unwrap();
    let after_page_move = illustrator::get_or_create_thread(&pdb2, "s1", "whole").unwrap();
    assert_eq!(whole.id, after_page_move.id);
}

#[test]
fn fr_l6_import_to_studio_copies_messages_into_a_new_tab() {
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
    let pdb = projects::open_db(&root, &pid).unwrap();
    pdb.execute(
        "INSERT INTO sources (id, kind, original_name, rel_path, status, added_at)
         VALUES ('s1','markdown','n.md','sources/s1/n.md','ready','now')",
        [],
    )
    .unwrap();

    let th = illustrator::get_or_create_thread(&pdb, "s1", "page:1").unwrap();
    for (i, (role, body)) in [("user", "q"), ("assistant", "a")].iter().enumerate() {
        pdb.execute(
            "INSERT INTO messages (id, thread_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, 'now')",
            params![format!("m{i}"), th.id, role, body],
        ).unwrap();
    }
    pdb.execute(
        "INSERT INTO illustrations
         (id, source_id, locator_key, lang, level, model, content, created_at)
         VALUES ('i1', 's1', 'page:1', 'ja', 'standard', 'model', 'page explanation', 'before')",
        [],
    )
    .unwrap();

    // Live activity remains private to Live Illustrator until the explicit
    // handoff command below is invoked.
    let before: i64 = pdb
        .query_row("SELECT count(*) FROM studio_tabs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, 0);

    let tab_id = illustrator::import_to_studio(
        &pdb,
        &ImportToStudioInput {
            project_id: pid.clone(),
            thread_id: th.id.clone(),
            mode: "new_tab".into(),
            target_tab_id: None,
        },
    )
    .unwrap();

    let studio_thread: String = pdb
        .query_row(
            "SELECT thread_id FROM studio_tabs WHERE id = ?1",
            [&tab_id],
            |r| r.get(0),
        )
        .unwrap();
    let n: i64 = pdb
        .query_row(
            "SELECT count(*) FROM messages WHERE thread_id = ?1",
            [&studio_thread],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        n, 3,
        "the saved explanation and both chat messages are handed off"
    );
    // The Illustrator thread is unchanged (copy, not move).
    let orig: i64 = pdb
        .query_row(
            "SELECT count(*) FROM messages WHERE thread_id = ?1",
            [&th.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orig, 2);
}
