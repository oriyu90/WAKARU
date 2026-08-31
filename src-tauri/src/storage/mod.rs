//! SQLite access (docs/03). Two databases: `app.db` (this module opens it) and one
//! `project.db` per project (opened by the projects service, Phase 1). Both use the
//! same pragmas and the same forward-only migration runner.

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::Path;

pub mod migrate;

pub const APP_SCHEMA_VERSION: &str = "1.0.0";

/// App-wide migrations, applied in array order. Names are `NNN_desc`; no gaps.
pub const APP_MIGRATIONS: &[(&str, &str)] =
    &[("001_init", include_str!("../../migrations/app/001_init.sql"))];

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

#[allow(dead_code)] // used by tests + Phase 1
pub fn open_in_memory() -> AppResult<Connection> {
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
    migrate::run(&conn, APP_MIGRATIONS, APP_SCHEMA_VERSION)?;
    Ok(conn)
}
