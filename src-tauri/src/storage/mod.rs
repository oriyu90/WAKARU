//! SQLite access (docs/03). Two databases: `app.db` (this module opens it) and one
//! `project.db` per project (opened by the projects service, Phase 1). Both use the
//! same pragmas and the same forward-only migration runner.

use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub mod migrate;

pub const APP_SCHEMA_VERSION: &str = "1.1.0";
pub const PROJECT_SCHEMA_VERSION: &str = "1.0.0";

/// App-wide migrations, applied in array order. Names are `NNN_desc`; no gaps.
pub const APP_MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_init",
        include_str!("../../migrations/app/001_init.sql"),
    ),
    (
        "002_ai_protocol",
        include_str!("../../migrations/app/002_ai_protocol.sql"),
    ),
];

/// Per-project database migrations.
pub const PROJECT_MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_init",
        include_str!("../../migrations/project/001_init.sql"),
    ),
    (
        "002_studio",
        include_str!("../../migrations/project/002_studio.sql"),
    ),
];

/// Open a `project.db`, run its migrations. Called when a project is opened.
pub fn open_project_db(path: &Path) -> AppResult<Connection> {
    let conn = open(path)?;
    adopt_legacy_project_schema(&conn)?;
    migrate::run(&conn, PROJECT_MIGRATIONS, PROJECT_SCHEMA_VERSION)?;
    Ok(conn)
}

/// Register the `sqlite-vec` extension once, before any connection is opened, so
/// every connection can use `vec0` virtual tables (docs/03 §4).
pub fn register_extensions() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    #[allow(clippy::missing_transmute_annotations)]
    ONCE.call_once(|| unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}

pub fn open(path: &Path) -> AppResult<Connection> {
    register_extensions();
    let conn = Connection::open(path)?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

#[allow(dead_code)] // used by tests + Phase 1
pub fn open_in_memory() -> AppResult<Connection> {
    register_extensions();
    let conn = Connection::open_in_memory()?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

fn apply_pragmas(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

/// Open `app.db`, verify integrity, run migrations. Called once at startup.
pub fn open_app_db(path: &Path) -> AppResult<Connection> {
    let conn = open(path)?;
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap_or_else(|_| "unknown".into());
    if ok != "ok" {
        return Err(AppError::new(
            "DB_CORRUPT",
            "error.db.corrupt",
            format!("integrity_check returned: {ok}"),
        ));
    }
    adopt_legacy_app_schema(&conn)?;
    migrate::run(&conn, APP_MIGRATIONS, APP_SCHEMA_VERSION)?;
    Ok(conn)
}

/// Builds before the forward-only runner was introduced created the complete
/// tables but no `schema_migrations` rows. Adopt only a recognisable complete
/// baseline, then let normal migrations add newer fields. This prevents an
/// upgrade from trying to recreate `projects` and aborting at launch.
fn adopt_legacy_app_schema(conn: &Connection) -> AppResult<()> {
    ensure_tracking_tables(conn)?;
    if table_exists(conn, "global_index")? {
        repair_legacy_global_index(conn)?;
    }
    if migration_applied(conn, "001_init")? || !table_exists(conn, "projects")? {
        return Ok(());
    }
    for required in [
        "settings",
        "ai_profiles",
        "model_roles",
        "whisper_models",
        "mcp_servers",
        "mcp_tool_policies",
        "global_index",
    ] {
        if !table_exists(conn, required)? {
            return Err(AppError::new(
                "MIGRATION_FAILED",
                "error.db.migration",
                format!("legacy app database is incomplete: missing {required}"),
            ));
        }
    }
    if !column_exists(conn, "ai_profiles", "json_schema")? {
        conn.execute(
            "ALTER TABLE ai_profiles ADD COLUMN json_schema INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    mark_migration_applied(conn, "001_init")?;
    if column_exists(conn, "ai_profiles", "protocol")? {
        mark_migration_applied(conn, "002_ai_protocol")?;
    }
    tracing::info!("adopted legacy app database baseline");
    Ok(())
}

/// Early builds created the cross-project FTS mirror without `body_raw`.
/// FTS5 virtual tables cannot add a column with `ALTER TABLE`, so rebuild the
/// mirror atomically and use the indexed body as the display fallback for any
/// rows that already exist. The per-project databases remain the source of
/// truth and future writes populate both fields independently.
fn repair_legacy_global_index(conn: &Connection) -> AppResult<()> {
    if column_exists(conn, "global_index", "body_raw")? {
        return Ok(());
    }
    for required in ["project_id", "source_id", "kind", "ref_id", "title", "body"] {
        if !column_exists(conn, "global_index", required)? {
            return Err(AppError::new(
                "MIGRATION_FAILED",
                "error.db.migration",
                format!("legacy global search index is missing column: {required}"),
            ));
        }
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        "CREATE VIRTUAL TABLE global_index_repaired USING fts5(
           project_id UNINDEXED,
           source_id  UNINDEXED,
           kind       UNINDEXED,
           ref_id     UNINDEXED,
           title,
           body,
           body_raw   UNINDEXED,
           tokenize = 'unicode61 remove_diacritics 2'
         );
         INSERT INTO global_index_repaired(
           rowid, project_id, source_id, kind, ref_id, title, body, body_raw
         )
         SELECT rowid, project_id, source_id, kind, ref_id, title, body, body
         FROM global_index;
         DROP TABLE global_index;
         ALTER TABLE global_index_repaired RENAME TO global_index;",
    )?;
    tx.commit()?;
    tracing::info!("repaired legacy global search index schema");
    Ok(())
}

fn adopt_legacy_project_schema(conn: &Connection) -> AppResult<()> {
    ensure_tracking_tables(conn)?;
    if !migration_applied(conn, "001_init")? && table_exists(conn, "sources")? {
        for required in ["documents", "chunks", "threads", "messages", "studio_tabs"] {
            if !table_exists(conn, required)? {
                return Err(AppError::new(
                    "MIGRATION_FAILED",
                    "error.db.migration",
                    format!("legacy project database is incomplete: missing {required}"),
                ));
            }
        }
        mark_migration_applied(conn, "001_init")?;
        tracing::info!("adopted legacy project database baseline");
    }
    if column_exists(conn, "studio_tabs", "scope")? && !migration_applied(conn, "002_studio")? {
        mark_migration_applied(conn, "002_studio")?;
    }
    Ok(())
}

fn ensure_tracking_tables(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );",
    )?;
    Ok(())
}

fn migration_applied(conn: &Connection, name: &str) -> AppResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM schema_migrations WHERE name = ?1",
            [name],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false))
}

fn mark_migration_applied(conn: &Connection, name: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (name, applied_at) VALUES (?1, ?2)",
        rusqlite::params![name, migrate::now_iso8601()],
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> AppResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
            [table],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false))
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> AppResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(names.iter().any(|name| name == column))
}

#[cfg(test)]
mod legacy_tests {
    use super::*;

    #[test]
    fn legacy_app_schema_is_adopted_and_upgraded_without_recreating_tables() {
        let conn = open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE projects(id TEXT PRIMARY KEY);
             CREATE TABLE settings(key TEXT PRIMARY KEY);
             CREATE TABLE ai_profiles(
                id TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL,
                api_key_ref TEXT, default_model TEXT, supports_vision INTEGER NOT NULL DEFAULT 0,
                supports_tools INTEGER NOT NULL DEFAULT 0, supports_embed INTEGER NOT NULL DEFAULT 0,
                extra_headers TEXT NOT NULL DEFAULT '{}', timeout_ms INTEGER NOT NULL DEFAULT 120000,
                created_at TEXT NOT NULL, last_ok_at TEXT
             );
             CREATE TABLE model_roles(role TEXT PRIMARY KEY);
             CREATE TABLE whisper_models(name TEXT PRIMARY KEY);
             CREATE TABLE mcp_servers(id TEXT PRIMARY KEY);
             CREATE TABLE mcp_tool_policies(server_id TEXT, tool_name TEXT);
             CREATE VIRTUAL TABLE global_index USING fts5(
                project_id UNINDEXED, source_id UNINDEXED, kind UNINDEXED,
                ref_id UNINDEXED, title, body,
                tokenize = 'unicode61 remove_diacritics 2'
             );
             INSERT INTO global_index(
                project_id, source_id, kind, ref_id, title, body
             ) VALUES ('p1', 's1', 'source', 'd1', 'Legacy', 'indexed text');",
        )
        .unwrap();

        adopt_legacy_app_schema(&conn).unwrap();
        migrate::run(&conn, APP_MIGRATIONS, APP_SCHEMA_VERSION).unwrap();
        assert!(column_exists(&conn, "ai_profiles", "json_schema").unwrap());
        assert!(column_exists(&conn, "ai_profiles", "protocol").unwrap());
        assert!(column_exists(&conn, "global_index", "body_raw").unwrap());
        let preserved: (String, String) = conn
            .query_row(
                "SELECT body, body_raw FROM global_index WHERE source_id = 's1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(preserved, ("indexed text".into(), "indexed text".into()));
        assert!(migration_applied(&conn, "001_init").unwrap());
        assert!(migration_applied(&conn, "002_ai_protocol").unwrap());
    }

    #[test]
    fn already_adopted_legacy_app_still_repairs_the_old_fts_shape() {
        let conn = open_in_memory().unwrap();
        ensure_tracking_tables(&conn).unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE global_index USING fts5(
                project_id UNINDEXED, source_id UNINDEXED, kind UNINDEXED,
                ref_id UNINDEXED, title, body,
                tokenize = 'unicode61 remove_diacritics 2'
             );
             INSERT INTO global_index(
                project_id, source_id, kind, ref_id, title, body
             ) VALUES ('p1', 's1', 'source', 'd1', 'Legacy', 'indexed text');",
        )
        .unwrap();
        mark_migration_applied(&conn, "001_init").unwrap();

        adopt_legacy_app_schema(&conn).unwrap();

        assert!(column_exists(&conn, "global_index", "body_raw").unwrap());
        let display: String = conn
            .query_row(
                "SELECT body_raw FROM global_index WHERE source_id = 's1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(display, "indexed text");
    }
}
