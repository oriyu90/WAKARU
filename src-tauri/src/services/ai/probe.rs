//! Capability detection (docs/05 §1.3). `GET /models` for reachability + list,
//! then small real probes for vision / tools / json_schema. Never fatal — a
//! failed probe just means "not supported".

use super::client::AiClient;
use crate::domain::ai::TestResult;
use crate::error::AppResult;
use serde_json::json;
use std::time::Instant;

// 1x1 transparent PNG.
const TINY_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

pub async fn probe(client: &AiClient, model: &str) -> AppResult<TestResult> {
    let start = Instant::now();
    let models = client.list_models().await.unwrap_or_default();
    let reachable = !models.is_empty() || model_ping(client, model).await;

    let mut result = TestResult {
        ok: reachable,
        models,
        latency_ms: start.elapsed().as_millis() as u32,
        supports_vision: false,
        supports_tools: false,
        supports_embed: false,
        json_schema: false,
        note: None,
    };
    if !reachable {
        result.note = Some("endpoint unreachable".into());
        return Ok(result);
    }

    result.supports_vision = probe_vision(client, model).await;
    result.supports_tools = probe_tools(client, model).await;
    result.json_schema = probe_json_schema(client, model).await;
    result.supports_embed = client
        .embeddings(model, &["ping".to_string()])
        .await
        .is_ok();

    Ok(result)
}

async fn model_ping(client: &AiClient, model: &str) -> bool {
    // A 1-token completion. If it doesn't error hard, the endpoint is alive.
    let cancel = tokio_util::sync::CancellationToken::new();
    client
        .chat_stream(
            model,
            json!([{ "role": "user", "content": "ping" }]),
            &json!({ "max_tokens": 1 }),
            &cancel,
            |_, _| {},
        )
        .await
        .is_ok()
}

async fn probe_vision(client: &AiClient, model: &str) -> bool {
    let cancel = tokio_util::sync::CancellationToken::new();
    client
        .chat_stream(
            model,
            json!([{
                "role": "user",
                "content": [
                    { "type": "text", "text": "one word colour?" },
                    { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{TINY_PNG}") } }
                ]
            }]),
            &json!({ "max_tokens": 3 }),
            &cancel,
            |_, _| {},
        )
        .await
        .is_ok()
}

async fn probe_tools(client: &AiClient, model: &str) -> bool {
    let cancel = tokio_util::sync::CancellationToken::new();
    client
        .chat_stream(
            model,
            json!([{ "role": "user", "content": "hi" }]),
            &json!({
                "max_tokens": 1,
                "tools": [{
                    "type": "function",
                    "function": { "name": "noop", "description": "no-op", "parameters": { "type": "object", "properties": {} } }
                }]
            }),
            &cancel,
            |_, _| {},
        )
        .await
        .is_ok()
}

async fn probe_json_schema(client: &AiClient, model: &str) -> bool {
    let cancel = tokio_util::sync::CancellationToken::new();
    client
        .chat_stream(
            model,
            json!([{ "role": "user", "content": "return {\"ok\":true}" }]),
            &json!({
                "max_tokens": 10,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": "probe",
                        "schema": { "type": "object", "properties": { "ok": { "type": "boolean" } }, "required": ["ok"] }
                    }
                }
            }),
            &cancel,
            |_, _| {},
        )
        .await
        .is_ok()
}
