//! Settings persistence (docs/07 §1). One JSON row in `app.db` `settings`
//! (`key='app'`). Missing / unknown fields fall back to `Settings::default()`,
//! so a settings file from a newer build still loads.

use crate::domain::settings::Settings;
use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};

const KEY: &str = "app";

pub fn get(app_db: &Connection) -> AppResult<Settings> {
    let raw: Option<String> = app_db
        .query_row("SELECT value FROM settings WHERE key = ?1", [KEY], |r| r.get(0))
        .optional()?;
    Ok(match raw {
        Some(json) => merge_defaults(&json),
        None => Settings::default(),
    })
}

pub fn update(app_db: &Connection, patch: serde_json::Value) -> AppResult<Settings> {
    let current = get(app_db)?;
    let mut base = serde_json::to_value(&current)?;
    deep_merge(&mut base, patch);
    let merged: Settings = serde_json::from_value(base).unwrap_or(current);
    app_db.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![KEY, serde_json::to_string(&merged)?],
    )?;
    Ok(merged)
}

/// Parse stored JSON over a default so partial / old shapes still work.
fn merge_defaults(json: &str) -> Settings {
    let defaults = serde_json::to_value(Settings::default()).unwrap();
    let stored: serde_json::Value = serde_json::from_str(json).unwrap_or(serde_json::json!({}));
    let mut merged = defaults;
    deep_merge(&mut merged, stored);
    serde_json::from_value(merged).unwrap_or_default()
}

fn deep_merge(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                deep_merge(b.entry(k).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, p) => *b = p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_app_db;

    #[test]
    fn round_trips_and_partial_patch_keeps_other_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let db = open_app_db(&tmp.path().join("app.db")).unwrap();

        let s0 = get(&db).unwrap();
        assert_eq!(s0.illustrator.default_level, "standard");

        let s1 = update(&db, serde_json::json!({ "illustrator": { "enabled": true } })).unwrap();
        assert!(s1.illustrator.enabled);
        assert_eq!(s1.illustrator.default_level, "standard"); // untouched

        let s2 = get(&db).unwrap();
        assert!(s2.illustrator.enabled);
    }

    #[test]
    fn unknown_stored_fields_do_not_break_load() {
        let tmp = tempfile::tempdir().unwrap();
        let db = open_app_db(&tmp.path().join("app.db")).unwrap();
        db.execute(
            "INSERT INTO settings (key, value) VALUES ('app', ?1)",
            [r#"{"general":{"startupView":"home"},"somethingNew":42}"#],
        )
        .unwrap();
        let s = get(&db).unwrap();
        assert_eq!(s.general.startup_view, "home");
        assert_eq!(s.ai_budget.context_tokens, 32768); // filled from defaults
    }
}
