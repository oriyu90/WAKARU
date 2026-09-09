//! AI connection profiles + role bindings (docs/03 §3, FR-A1..A5). API keys live
//! in the OS keychain (I-4); `app.db` only holds a reference flag.

use crate::domain::ai::*;
use crate::error::{AppError, AppResult};
use crate::services::ai::client::ensure_api_version_path;
use crate::storage::migrate::now_iso8601;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

const KEYRING_SERVICE: &str = "com.yukiorita.wakaru";

fn keyring_entry(profile_id: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, &format!("ai_profile:{profile_id}"))
        .map_err(|e| AppError::internal(format!("keyring: {e}")))
}

pub fn get_key(profile_id: &str) -> Option<String> {
    keyring_entry(profile_id).ok()?.get_password().ok()
}

fn set_key(profile_id: &str, key: &str) -> AppResult<()> {
    let entry = keyring_entry(profile_id)?;
    if key.is_empty() {
        let _ = entry.delete_credential();
    } else {
        entry
            .set_password(key)
            .map_err(|e| AppError::internal(format!("keyring set: {e}")))?;
    }
    Ok(())
}

fn delete_key(profile_id: &str) {
    if let Ok(e) = keyring_entry(profile_id) {
        let _ = e.delete_credential();
    }
}

fn row_to_profile(r: &rusqlite::Row) -> rusqlite::Result<AiProfile> {
    let id: String = r.get("id")?;
    Ok(AiProfile {
        has_key: r.get::<_, Option<String>>("api_key_ref")?.is_some(),
        id,
        name: r.get("name")?,
        base_url: r.get("base_url")?,
        protocol: match r.get::<_, String>("protocol")?.as_str() {
            "anthropic" => ApiProtocol::Anthropic,
            _ => ApiProtocol::Openai,
        },
        default_model: r.get("default_model")?,
        supports_vision: r.get::<_, i64>("supports_vision")? != 0,
        supports_tools: r.get::<_, i64>("supports_tools")? != 0,
        supports_embed: r.get::<_, i64>("supports_embed")? != 0,
        json_schema: r.get::<_, i64>("json_schema")? != 0,
        extra_headers: serde_json::from_str(&r.get::<_, String>("extra_headers")?)
            .unwrap_or(serde_json::json!({})),
        timeout_ms: r.get::<_, i64>("timeout_ms")? as u32,
        created_at: r.get("created_at")?,
        last_ok_at: r.get("last_ok_at")?,
    })
}

pub fn list(app_db: &Connection) -> AppResult<Vec<AiProfile>> {
    let mut stmt = app_db.prepare("SELECT * FROM ai_profiles ORDER BY created_at")?;
    let rows: Vec<AiProfile> = stmt
        .query_map([], row_to_profile)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub fn get(app_db: &Connection, id: &str) -> AppResult<AiProfile> {
    app_db
        .query_row(
            "SELECT * FROM ai_profiles WHERE id = ?1",
            [id],
            row_to_profile,
        )
        .optional()?
        .ok_or_else(|| AppError::new("AI_PROFILE_NOT_FOUND", "error.ai.profileNotFound", id))
}

pub fn upsert(app_db: &Connection, input: AiProfileInput) -> AppResult<AiProfile> {
    let name = input.name.trim();
    let base_url = normalise_base_url(&input.base_url)?;
    if name.is_empty() || base_url.is_empty() {
        return Err(AppError::new(
            "AI_PROFILE_INVALID",
            "error.ai.profileInvalid",
            "name and base URL are required",
        ));
    }
    let headers = validate_extra_headers(input.extra_headers)?.to_string();
    let timeout = input.timeout_ms.unwrap_or(120_000).clamp(1000, 1_800_000);
    let default_model = input
        .default_model
        .and_then(|m| (!m.trim().is_empty()).then(|| m.trim().to_string()));

    let id = match input.id {
        Some(id) => {
            app_db.execute(
                "UPDATE ai_profiles
                   SET name=?2, base_url=?3, protocol=?4, default_model=?5, extra_headers=?6, timeout_ms=?7
                 WHERE id=?1",
                params![
                    id,
                    name,
                    base_url,
                    input.protocol.as_str(),
                    default_model,
                    headers,
                    timeout as i64
                ],
            )?;
            id
        }
        None => {
            let id = Uuid::now_v7().to_string();
            app_db.execute(
                "INSERT INTO ai_profiles
                   (id, name, base_url, protocol, default_model, extra_headers, timeout_ms, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id,
                    name,
                    base_url,
                    input.protocol.as_str(),
                    default_model,
                    headers,
                    timeout as i64,
                    now_iso8601()
                ],
            )?;
            id
        }
    };

    if let Some(key) = input.api_key {
        set_key(&id, &key)?;
        let has = if key.is_empty() {
            None
        } else {
            Some(id.clone())
        };
        app_db.execute(
            "UPDATE ai_profiles SET api_key_ref = ?2 WHERE id = ?1",
            params![id, has],
        )?;
    }

    get(app_db, &id)
}

fn validate_extra_headers(value: Option<serde_json::Value>) -> AppResult<serde_json::Value> {
    let value = value.unwrap_or_else(|| serde_json::json!({}));
    let object = value.as_object().ok_or_else(|| {
        AppError::new(
            "AI_PROFILE_INVALID",
            "error.ai.profileInvalid",
            "extra headers must be a JSON object",
        )
    })?;
    let mut clean = serde_json::Map::new();
    for (name, value) in object {
        let lower = name.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "authorization"
                | "x-api-key"
                | "anthropic-version"
                | "host"
                | "content-length"
                | "transfer-encoding"
        ) {
            return Err(AppError::new(
                "AI_PROFILE_INVALID",
                "error.ai.profileInvalid",
                format!("{name} must be configured using the API key/protocol fields"),
            ));
        }
        reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            AppError::new(
                "AI_PROFILE_INVALID",
                "error.ai.profileInvalid",
                format!("invalid extra header name: {name}"),
            )
        })?;
        let text = value.as_str().ok_or_else(|| {
            AppError::new(
                "AI_PROFILE_INVALID",
                "error.ai.profileInvalid",
                format!("extra header {name} must be a string"),
            )
        })?;
        reqwest::header::HeaderValue::from_str(text).map_err(|_| {
            AppError::new(
                "AI_PROFILE_INVALID",
                "error.ai.profileInvalid",
                format!("invalid extra header value: {name}"),
            )
        })?;
        clean.insert(name.clone(), serde_json::Value::String(text.to_string()));
    }
    Ok(serde_json::Value::Object(clean))
}

fn normalise_base_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| {
        AppError::new(
            "AI_PROFILE_INVALID",
            "error.ai.profileInvalid",
            "base URL must be an absolute http(s) URL",
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AppError::new(
            "AI_PROFILE_INVALID",
            "error.ai.profileInvalid",
            "base URL must be http(s), without credentials, query, or fragment",
        ));
    }
    // Persist the completed form so the stored value matches what is actually
    // requested and what the Settings list shows (see `ensure_api_version_path`).
    Ok(ensure_api_version_path(trimmed))
}

pub fn delete(app_db: &Connection, id: &str) -> AppResult<()> {
    get(app_db, id)?;
    app_db.execute("DELETE FROM ai_profiles WHERE id = ?1", [id])?;
    delete_key(id);
    Ok(())
}

pub fn set_capabilities(app_db: &Connection, id: &str, r: &TestResult) -> AppResult<()> {
    app_db.execute(
        "UPDATE ai_profiles
           SET supports_vision=?2, supports_tools=?3, supports_embed=?4, json_schema=?5,
               last_ok_at = CASE WHEN ?6 THEN ?7 ELSE last_ok_at END
         WHERE id=?1",
        params![
            id,
            r.supports_vision as i64,
            r.supports_tools as i64,
            r.supports_embed as i64,
            r.json_schema as i64,
            r.ok,
            now_iso8601()
        ],
    )?;
    Ok(())
}

// ───────────── role bindings ─────────────

pub fn get_bindings(app_db: &Connection) -> AppResult<RoleBindings> {
    let mut stmt = app_db.prepare("SELECT role, profile_id, model, params FROM model_roles")?;
    let mut b = RoleBindings {
        chat: None,
        vision: None,
        embedding: None,
        organizer: None,
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            RoleBinding {
                profile_id: r.get(1)?,
                model: r.get(2)?,
                params: serde_json::from_str(&r.get::<_, String>(3)?)
                    .unwrap_or(serde_json::json!({})),
            },
        ))
    })?;
    for row in rows {
        let (role, binding) = row?;
        match role.as_str() {
            "chat" => b.chat = Some(binding),
            "vision" => b.vision = Some(binding),
            "embedding" => b.embedding = Some(binding),
            "organizer" => b.organizer = Some(binding),
            _ => {}
        }
    }
    Ok(b)
}

pub fn set_binding(
    app_db: &Connection,
    role: Role,
    profile_id: &str,
    model: &str,
    params: serde_json::Value,
) -> AppResult<()> {
    get(app_db, profile_id)?;
    let role_str = match role {
        Role::Chat => "chat",
        Role::Vision => "vision",
        Role::Embedding => "embedding",
        Role::Organizer => "organizer",
    };
    app_db.execute(
        "INSERT INTO model_roles (role, profile_id, model, params) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(role) DO UPDATE SET profile_id=excluded.profile_id, model=excluded.model, params=excluded.params",
        params![role_str, profile_id, model, params.to_string()],
    )?;
    Ok(())
}

pub fn clear_binding(app_db: &Connection, role: Role) -> AppResult<()> {
    let role_str = match role {
        Role::Chat => "chat",
        Role::Vision => "vision",
        Role::Embedding => "embedding",
        Role::Organizer => "organizer",
    };
    app_db.execute("DELETE FROM model_roles WHERE role = ?1", [role_str])?;
    Ok(())
}

/// Resolve a role to (base_url, key, headers, timeout, model, params), applying
/// the `organizer -> chat` fallback (docs/07 §1).
pub fn resolve(app_db: &Connection, role: Role) -> AppResult<Option<ResolvedRole>> {
    let b = get_bindings(app_db)?;
    let binding = match role {
        Role::Chat => b.chat,
        Role::Vision => b.vision,
        Role::Embedding => b.embedding,
        Role::Organizer => b.organizer.or(b.chat),
    };
    let Some(binding) = binding else {
        return Ok(None);
    };
    let profile = get(app_db, &binding.profile_id)?;
    let headers = profile
        .extra_headers
        .as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(ResolvedRole {
        base_url: profile.base_url,
        protocol: profile.protocol,
        api_key: get_key(&binding.profile_id),
        extra_headers: headers,
        timeout_ms: profile.timeout_ms,
        model: binding.model,
        params: binding.params,
    }))
}

#[derive(Debug, Clone)]
pub struct ResolvedRole {
    pub base_url: String,
    pub protocol: ApiProtocol,
    pub api_key: Option<String>,
    pub extra_headers: Vec<(String, String)>,
    pub timeout_ms: u32,
    pub model: String,
    pub params: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_validation_accepts_http_and_rejects_unsafe_shapes() {
        assert_eq!(
            normalise_base_url(" https://api.example.com/v1/ ").unwrap(),
            "https://api.example.com/v1"
        );
        // A bare host is completed to `/v1` (the LM Studio "no reply" fix).
        assert_eq!(
            normalise_base_url("http://192.168.0.114:1234").unwrap(),
            "http://192.168.0.114:1234/v1"
        );
        for invalid in [
            "file:///tmp/api",
            "https://user:pass@example.com/v1",
            "https://example.com/v1?key=secret",
            "not-a-url",
        ] {
            assert!(normalise_base_url(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn extra_headers_reject_auth_overrides_and_invalid_values() {
        assert!(validate_extra_headers(Some(serde_json::json!({
            "X-Tenant": "local"
        })))
        .is_ok());
        for invalid in [
            serde_json::json!({"Authorization": "Basic secret"}),
            serde_json::json!({"bad header": "x"}),
            serde_json::json!({"X-Number": 1}),
        ] {
            assert!(validate_extra_headers(Some(invalid)).is_err());
        }
    }

    #[test]
    fn anthropic_profile_round_trips_through_migrated_database() {
        let db = crate::storage::open_in_memory().unwrap();
        crate::storage::migrate::run(
            &db,
            crate::storage::APP_MIGRATIONS,
            crate::storage::APP_SCHEMA_VERSION,
        )
        .unwrap();
        let profile = upsert(
            &db,
            AiProfileInput {
                id: None,
                name: "Claude gateway".into(),
                base_url: "https://api.example.com/v1/".into(),
                protocol: ApiProtocol::Anthropic,
                api_key: None,
                default_model: Some("claude-test".into()),
                extra_headers: None,
                timeout_ms: None,
            },
        )
        .unwrap();
        assert_eq!(profile.protocol, ApiProtocol::Anthropic);
        assert_eq!(profile.base_url, "https://api.example.com/v1");
        assert_eq!(
            get(&db, &profile.id).unwrap().protocol,
            ApiProtocol::Anthropic
        );
    }
}
