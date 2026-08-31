//! Forward-only migration runner (docs/03 §6.3). Each migration is one transaction;
//! on failure the whole migration rolls back and the DB stays at its old version.

use crate::error::{AppError, AppResult};
use rusqlite::Connection;

const APPLIED_TABLE: &str = "schema_migrations";

pub fn run(conn: &Connection, migrations: &[(&str, &str)], target_version: &str) -> AppResult<()> {
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {APPLIED_TABLE} (
            name       TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS schema_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );"
    ))?;

    for (name, sql) in migrations {
        let already: bool = conn.query_row(
            &format!("SELECT 1 FROM {APPLIED_TABLE} WHERE name = ?1"),
            [name],
            |_| Ok(true),
        ).unwrap_or(false);
        if already {
            continue;
        }

        conn.execute_batch("BEGIN")?;
        let step = (|| -> AppResult<()> {
            conn.execute_batch(sql)?;
            conn.execute(
                &format!("INSERT INTO {APPLIED_TABLE} (name, applied_at) VALUES (?1, ?2)"),
                rusqlite::params![name, now_iso8601()],
            )?;
            Ok(())
        })();
        match step {
            Ok(()) => {
                conn.execute_batch("COMMIT")?;
                tracing::info!(migration = name, "applied");
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(AppError::new(
                    "MIGRATION_FAILED",
                    "error.db.migration",
                    format!("migration {name} failed: {e}"),
                ));
            }
        }
    }

    conn.execute(
        "INSERT INTO schema_meta (key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [target_version],
    )?;
    Ok(())
}

pub fn now_iso8601() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_in_memory;

    #[test]
    fn runs_from_empty_and_is_idempotent() {
        let conn = open_in_memory().unwrap();
        let m: &[(&str, &str)] = &[
            ("001_a", "CREATE TABLE a (id INTEGER PRIMARY KEY);"),
            ("002_b", "CREATE TABLE b (id INTEGER PRIMARY KEY);"),
        ];
        run(&conn, m, "1.0.0").unwrap();
        run(&conn, m, "1.0.0").unwrap(); // second run is a no-op

        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('a','b')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 2);

        let v: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key='schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, "1.0.0");
    }

    #[test]
    fn failed_migration_rolls_back() {
        let conn = open_in_memory().unwrap();
        let m: &[(&str, &str)] = &[
            ("001_ok", "CREATE TABLE ok (id INTEGER PRIMARY KEY);"),
            ("002_bad", "CREATE TABLE bad (id INTEGER PRIMARY KEY); INSERT INTO nope VALUES (1);"),
        ];
        let err = run(&conn, m, "1.0.0").unwrap_err();
        assert_eq!(err.code, "MIGRATION_FAILED");

        // 001 stuck; 002 fully rolled back — `bad` must not exist.
        let has_bad: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='bad'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(!has_bad);
    }
}
