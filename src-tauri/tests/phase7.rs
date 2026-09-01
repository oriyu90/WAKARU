//! Phase 7 integration (docs/09 AC-7-*). The sandbox path/exec guarantees
//! (AC-7-5..7-10) are unit-tested in `services::sandbox`; live MCP transport
//! (AC-7-1/7-11/7-12) is exercised manually. Here we lock the server registry
//! and the tool-policy record/revoke contract (AC-7-4) and the `<slug>__<tool>`
//! namespacing (docs/05 §6.3).

use wakaru_lib::services::mcp;
use wakaru_lib::storage;

fn app_db() -> (tempfile::TempDir, rusqlite::Connection) {
    let tmp = tempfile::tempdir().unwrap();
    let db = storage::open_app_db(&tmp.path().join("app.db")).unwrap();
    (tmp, db)
}

#[test]
fn slug_is_alnum_and_underscore_only() {
    assert_eq!(mcp::slugify("My Cool Server!"), "my_cool_server");
    assert_eq!(mcp::slugify("  --  "), "server");
    assert_eq!(mcp::slugify("files.fs"), "files_fs");
}

#[test]
fn ac_7_4_tool_policy_is_recorded_and_revocable() {
    let (_tmp, db) = app_db();
    db.execute(
        "INSERT INTO mcp_servers (id, name, transport, command, args, env, enabled, created_at)
         VALUES ('srv1', 'Files FS', 'stdio', '/usr/bin/true', '[]', '{}', 1, 'now')",
        [],
    )
    .unwrap();

    // Default: nothing stored -> the loop treats it as "ask".
    assert!(mcp::policy_map(&db).unwrap().is_empty());

    // "Always allow" is persisted, keyed by <slug>__<tool>.
    mcp::set_policy(&db, "srv1", "read_file", "always_allow").unwrap();
    let map = mcp::policy_map(&db).unwrap();
    let qualified = format!(
        "{}__{}",
        mcp::server_slug("Files FS", "srv1"),
        mcp::tool_alias("read_file")
    );
    assert_eq!(
        map.get(&qualified).map(String::as_str),
        Some("always_allow")
    );

    // Revoked from settings -> back to ask.
    mcp::set_policy(&db, "srv1", "read_file", "ask").unwrap();
    assert_eq!(
        mcp::policy_map(&db)
            .unwrap()
            .get(&qualified)
            .map(String::as_str),
        Some("ask")
    );

    assert!(mcp::set_policy(&db, "srv1", "read_file", "bogus").is_err());
}

#[test]
fn deleting_a_server_cascades_its_policies() {
    let (_tmp, db) = app_db();
    db.execute(
        "INSERT INTO mcp_servers (id, name, transport, command, args, env, enabled, created_at)
         VALUES ('s', 'S', 'stdio', '/bin/true', '[]', '{}', 1, 'now')",
        [],
    )
    .unwrap();
    mcp::set_policy(&db, "s", "t", "deny").unwrap();
    db.execute("DELETE FROM mcp_servers WHERE id = 's'", [])
        .unwrap();
    assert!(mcp::policy_map(&db).unwrap().is_empty());
    assert!(mcp::list_servers(&db).unwrap().is_empty());
}

#[tokio::test]
async fn http_transport_is_refused_this_release() {
    // DECISIONS D-14: stdio only for v0.0.0.
    let row = mcp::McpServerRow {
        id: "x".into(),
        name: "Remote".into(),
        transport: "http".into(),
        command: None,
        args: vec![],
        url: Some("https://example.com/mcp".into()),
        env: serde_json::Map::new(),
    };
    let err = mcp::connect(row, &std::collections::HashMap::new())
        .await
        .unwrap_err();
    assert_eq!(err.code, "MCP_TRANSPORT_UNSUPPORTED");
}
