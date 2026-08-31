//! AI Gateway (docs/05). One OpenAI-compatible client, profile/role management,
//! capability probing, and the streaming primitive that Live Illustrator (P4)
//! and Studio (P6) build on.

pub mod client;
pub mod probe;
pub mod profiles;

use crate::domain::ai::*;
use crate::error::{AppError, AppResult};
use client::AiClient;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Live streams, keyed by `stream_id`, so `*_cancel` can stop one.
#[derive(Default)]
pub struct StreamRegistry {
    inner: Mutex<HashMap<String, CancellationToken>>,
}

impl StreamRegistry {
    pub fn start(&self) -> (String, CancellationToken) {
        let id = Uuid::now_v7().to_string();
        let token = CancellationToken::new();
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), token.clone());
        (id, token)
    }
    /// Register a token under a caller-chosen key (Studio uses `studio:<tabId>`
    /// so `studio_cancel` can stop a running tool loop it never saw an id for).
    /// Any existing token under the key is replaced.
    pub fn start_keyed(&self, key: &str) -> CancellationToken {
        let token = CancellationToken::new();
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string(), token.clone());
        token
    }
    pub fn cancel(&self, id: &str) -> bool {
        if let Some(t) = self.inner.lock().unwrap_or_else(|e| e.into_inner()).get(id) {
            t.cancel();
            true
        } else {
            false
        }
    }
    pub fn finish(&self, id: &str) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
    }
}

/// `ai_test_profile` — probe reachability + capabilities and persist them.
pub async fn test_profile(
    app_db_path: &std::path::Path,
    profile_id: &str,
) -> AppResult<TestResult> {
    // Read the profile with a short-lived connection (this runs off the command thread).
    let (base_url, protocol, headers, timeout, model) = {
        let conn = crate::storage::open(app_db_path)?;
        let p = profiles::get(&conn, profile_id)?;
        let headers: Vec<(String, String)> = p
            .extra_headers
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        (
            p.base_url,
            p.protocol,
            headers,
            p.timeout_ms,
            p.default_model.unwrap_or_else(|| "gpt-4o-mini".into()),
        )
    };
    let key = profiles::get_key(profile_id);
    let cl = AiClient::new(protocol, &base_url, key, headers, timeout)?;
    let result = probe::probe(&cl, &model).await?;

    let conn = crate::storage::open(app_db_path)?;
    profiles::set_capabilities(&conn, profile_id, &result)?;
    Ok(result)
}

/// Stream a chat completion for `role`, emitting `stream://delta|done|error`.
/// Returns the `stream_id` immediately-ish (it awaits the whole stream here; the
/// command wrapper spawns it). `messages` is the OpenAI `messages` array.
#[allow(clippy::too_many_arguments)]
pub async fn stream_chat(
    app: &AppHandle,
    reg: &StreamRegistry,
    resolved: profiles::ResolvedRole,
    messages: serde_json::Value,
    stream_id: String,
    token: CancellationToken,
) {
    let emit_err = |e: AppError| {
        let _ = app.emit(
            "stream://error",
            StreamError {
                stream_id: stream_id.clone(),
                error: e,
            },
        );
    };

    let client = match AiClient::new(
        resolved.protocol,
        &resolved.base_url,
        resolved.api_key,
        resolved.extra_headers,
        resolved.timeout_ms,
    ) {
        Ok(c) => c,
        Err(e) => {
            emit_err(e);
            reg.finish(&stream_id);
            return;
        }
    };

    let sid = stream_id.clone();
    let app2 = app.clone();
    let res = client
        .chat_stream(
            &resolved.model,
            messages,
            &resolved.params,
            &token,
            move |kind, text| {
                let _ = app2.emit(
                    "stream://delta",
                    StreamDelta {
                        stream_id: sid.clone(),
                        kind: kind.to_string(),
                        text: text.to_string(),
                    },
                );
            },
        )
        .await;

    match res {
        Ok((usage, truncated, _)) => {
            let _ = app.emit(
                "stream://done",
                StreamDone {
                    stream_id: stream_id.clone(),
                    cancelled: token.is_cancelled(),
                    truncated,
                    usage,
                },
            );
        }
        Err(e) => emit_err(e),
    }
    reg.finish(&stream_id);
}

/// e5-family models need `query: ` / `passage: ` prefixes (docs/05 §2.1).
pub fn e5_prefix(model: &str, is_query: bool) -> &'static str {
    if model.to_lowercase().contains("e5") {
        if is_query {
            "query: "
        } else {
            "passage: "
        }
    } else {
        ""
    }
}

/// Compute embeddings for `texts` through the `embedding` role. Returns `None`
/// when no embedding endpoint is configured (search degrades to FTS — I-2).
/// `app_db` is only touched synchronously (before any await).
pub async fn embed(
    app_db: &Connection,
    texts: &[String],
    is_query: bool,
) -> AppResult<Option<(String, Vec<Vec<f32>>)>> {
    let Some(role) = profiles::resolve(app_db, Role::Embedding)? else {
        return Ok(None);
    };
    Ok(Some(embed_with(role, texts, is_query).await?))
}

/// Same, but with the role already resolved — used where a `Connection` must not
/// be held across the await (Tauri async commands require `Send`).
pub async fn embed_with(
    role: profiles::ResolvedRole,
    texts: &[String],
    is_query: bool,
) -> AppResult<(String, Vec<Vec<f32>>)> {
    let prefix = e5_prefix(&role.model, is_query);
    let inputs: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
    let client = AiClient::new(
        role.protocol,
        &role.base_url,
        role.api_key,
        role.extra_headers,
        role.timeout_ms,
    )?;
    let vectors = client.embeddings(&role.model, &inputs).await?;
    Ok((role.model, vectors))
}
