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

    // Distinguish a transport failure (nothing answered — wrong host/port, a proxy
    // in the way, or macOS local-network permission) from an HTTP response we can
    // read (the endpoint is up; `/models` just isn't usable here). Only the first
    // means "unreachable", and it must report *why* instead of a bare string.
    let mut soft_note: Option<String> = None;
    let models = match client.list_models().await {
        Ok(models) => models,
        Err(err) if err.code == "AI_NETWORK" => {
            let mut result = empty_result(start);
            result.note = Some(transport_hint(&err.message));
            return Ok(result);
        }
        Err(err) => {
            // e.g. 404 (no `/models` route) or 401 (key rejected for listing).
            soft_note = Some(err.message.clone());
            Vec::new()
        }
    };

    // A catalogue response proves only that authentication and routing work.
    // MLXBar can list a model that was manually stopped; a real generation then
    // fails with MODEL_NOT_LOADED. Probe the selected model before reporting OK.
    let generation = if model.trim().is_empty() {
        None
    } else {
        Some(model_ping(client, model).await)
    };
    let reachable = matches!(generation, Some(Ok(())));

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
        let mut note = match generation {
            Some(Err(err)) if err.code == "AI_MODEL_NOT_LOADED" => "AI_MODEL_NOT_LOADED".into(),
            Some(Err(err)) if err.code == "AI_AUTH" => "AI_AUTH".into(),
            Some(Err(err)) => err.message,
            None => "AI_MODEL_NOT_SET".into(),
            Some(Ok(())) => unreachable!(),
        };
        if let Some(list_error) = soft_note {
            note.push_str(&format!("; model list: {list_error}"));
        }
        // The most common misconfiguration for a local OpenAI-compatible server
        // (LM Studio, Ollama's OpenAI shim, …) is a base URL missing the `/v1`
        // path segment — `/models` and `/chat/completions` then 404.
        if !client.base_url().contains("/v1") {
            note.push_str(
                " — the base URL has no `/v1` path segment; most OpenAI-compatible \
                 servers expect it (e.g. http://host:1234/v1).",
            );
        }
        result.note = Some(note);
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

fn empty_result(start: Instant) -> TestResult {
    TestResult {
        ok: false,
        models: Vec::new(),
        latency_ms: start.elapsed().as_millis() as u32,
        supports_vision: false,
        supports_tools: false,
        supports_embed: false,
        json_schema: false,
        note: None,
    }
}

/// Turn a raw reqwest transport error into a short, actionable line. `raw` is
/// already secret-free (it is a connection-level error, not a response body).
fn transport_hint(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    let cause = if lower.contains("dns") || lower.contains("resolve") {
        "host name did not resolve"
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "no response before the timeout"
    } else if lower.contains("refused") {
        "connection refused"
    } else if lower.contains("certificate") || lower.contains("tls") {
        "TLS handshake failed"
    } else {
        "could not connect"
    };
    format!(
        "{cause}. Check the base URL and port, that the server is running, that no HTTP proxy \
         is intercepting LAN traffic, and — on macOS — that WAKARU is allowed under \
         System Settings › Privacy & Security › Local Network."
    )
}

async fn model_ping(client: &AiClient, model: &str) -> AppResult<()> {
    // A bounded generation confirms that this exact model can run. Reasoning
    // models may spend the entire tiny budget in hidden thought, so a clean
    // stream with a real delta still proves availability even if length-limited.
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut saw_delta = false;
    let _ = client
        .chat_stream(
            model,
            json!([{ "role": "user", "content": "Reply OK." }]),
            &json!({ "max_tokens": 64 }),
            &cancel,
            |_, text| saw_delta |= !text.is_empty(),
        )
        .await?;
    if !saw_delta {
        return Err(crate::error::AppError::new(
            "AI_BAD_RESPONSE",
            "error.ai.badResponse",
            "model probe produced no output",
        ));
    }
    Ok(())
}

async fn probe_vision(client: &AiClient, model: &str) -> bool {
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut output = String::new();
    let result = client
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
            |kind, delta| {
                if kind == "text" {
                    output.push_str(delta);
                }
            },
        )
        .await;
    result.is_ok() && !output.trim().is_empty()
}

async fn probe_tools(client: &AiClient, model: &str) -> bool {
    let cancel = tokio_util::sync::CancellationToken::new();
    client
        .chat_stream(
            model,
            json!([{ "role": "user", "content": "Call the noop tool now." }]),
            &json!({
                // Some OpenAI-compatible local models emit a short reasoning
                // prelude before their first tool-call fragment. Sixteen tokens
                // can therefore end the probe with `finish_reason: length`
                // even though the model does support tools. Keep this bounded,
                // but leave sufficient room for that prelude and the call.
                "max_tokens": 128,
                "tool_choice": "required",
                "tools": [{
                    "type": "function",
                    "function": { "name": "noop", "description": "no-op", "parameters": { "type": "object", "properties": {} } }
                }]
            }),
            &cancel,
            |_, _| {},
        )
        .await
        .map(|(_, _, calls)| calls.iter().any(|call| call.name == "noop"))
        .unwrap_or(false)
}

async fn probe_json_schema(client: &AiClient, model: &str) -> bool {
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut output = String::new();
    let result = client
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
            |kind, delta| {
                if kind == "text" {
                    output.push_str(delta);
                }
            },
        )
        .await;
    result.is_ok()
        && serde_json::from_str::<serde_json::Value>(output.trim())
            .ok()
            .and_then(|v| v.get("ok").and_then(serde_json::Value::as_bool))
            == Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ai::ApiProtocol;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn transport_hint_is_specific_and_actionable() {
        assert!(
            transport_hint("tcp connect error: Connection refused (os error 61)")
                .starts_with("connection refused")
        );
        assert!(transport_hint("error trying to connect: dns error")
            .starts_with("host name did not resolve"));
        for raw in [
            "operation timed out",
            "request or response body error",
            "invalid peer certificate",
        ] {
            let hint = transport_hint(raw);
            assert!(
                hint.contains("Local Network"),
                "no LAN guidance in {hint:?}"
            );
            assert!(hint.contains("proxy"), "no proxy guidance in {hint:?}");
        }
    }

    #[tokio::test]
    async fn probe_reports_the_transport_reason_without_retrying() {
        // Bind then drop so the port is closed -> connection refused, fast.
        let addr = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap()
        };
        let client = AiClient::new(
            ApiProtocol::Openai,
            &format!("http://{addr}/v1"),
            Some("secret-test-key".into()),
            Vec::new(),
            2_000,
        )
        .unwrap();

        let started = Instant::now();
        let result = probe(&client, "any-model").await.unwrap();
        assert!(!result.ok);
        let note = result.note.unwrap_or_default();
        assert!(note.contains("Local Network"), "note was {note:?}");
        assert!(
            !note.contains("secret-test-key"),
            "note leaked the key: {note:?}"
        );
        // No 1s+2s retry back-off on a dead endpoint.
        assert!(started.elapsed() < Duration::from_secs(2), "probe retried");
    }

    #[tokio::test]
    async fn listed_but_stopped_mlxbar_model_fails_generation_probe() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0u8; 8192];
                let size = socket.read(&mut request).unwrap();
                let route = String::from_utf8_lossy(&request[..size]);
                let (status, body) = if route.contains("GET /v1/models") {
                    (
                        "200 OK",
                        r#"{"data":[{"id":"Qwen3.8-27B-MLX-4bit","loaded":false}]}"#,
                    )
                } else {
                    (
                        "409 Conflict",
                        r#"{"error":{"code":"MODEL_NOT_LOADED","message":"stopped"}}"#,
                    )
                };
                write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        let client = AiClient::new(
            ApiProtocol::Openai,
            &format!("http://{addr}/v1"),
            None,
            vec![],
            2_000,
        )
        .unwrap();
        let result = probe(&client, "Qwen3.8-27B-MLX-4bit").await.unwrap();
        assert_eq!(result.models, vec!["Qwen3.8-27B-MLX-4bit"]);
        assert!(!result.ok);
        assert_eq!(result.note.as_deref(), Some("AI_MODEL_NOT_LOADED"));
        server.join().unwrap();
    }
}
