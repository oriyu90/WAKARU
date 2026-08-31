//! The one OpenAI-compatible HTTP client (I-1 — no vendor SDK). Async reqwest on
//! top of `rustls`. Handles `/models`, `/embeddings` and streaming
//! `/chat/completions`, with the retry policy from docs/05 §1.5.

use crate::domain::ai::TokenUsage;
use crate::error::{AppError, AppResult};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct AiClient {
    http: reqwest::Client,
    base: String,
    key: Option<String>,
    extra_headers: Vec<(String, String)>,
}

impl AiClient {
    pub fn new(base_url: &str, key: Option<String>, extra_headers: Vec<(String, String)>, timeout_ms: u32) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(1000) as u64))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| AppError::internal(format!("http client: {e}")))?;
        Ok(Self {
            http,
            base: base_url.trim_end_matches('/').to_string(),
            key: key.filter(|k| !k.is_empty()),
            extra_headers,
        })
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut rb = self.http.request(method, format!("{}{path}", self.base));
        if let Some(k) = &self.key {
            rb = rb.bearer_auth(k);
        }
        for (h, v) in &self.extra_headers {
            rb = rb.header(h.as_str(), v.as_str());
        }
        rb
    }

    pub async fn list_models(&self) -> AppResult<Vec<String>> {
        let resp = self
            .req(reqwest::Method::GET, "/models")
            .send()
            .await
            .map_err(net_err)?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| classify(status.as_u16(), e.to_string()))?;
        if !status.is_success() {
            return Err(classify(status.as_u16(), body.to_string()));
        }
        let models = body
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    /// Batch embeddings. Caller is responsible for the e5 `query:`/`passage:`
    /// prefixes (docs/05 §2.1).
    pub async fn embeddings(&self, model: &str, inputs: &[String]) -> AppResult<Vec<Vec<f32>>> {
        let body = json!({ "model": model, "input": inputs });
        let resp = retry(|| async {
            self.req(reqwest::Method::POST, "/embeddings").json(&body).send().await
        })
        .await?;
        let status = resp.status();
        let v: Value = resp.json().await.map_err(|e| classify(status.as_u16(), e.to_string()))?;
        if !status.is_success() {
            return Err(classify(status.as_u16(), v.to_string()));
        }
        let data = v.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
            AppError::new("AI_BAD_RESPONSE", "error.ai.badResponse", "no data[] in embeddings response")
        })?;
        let mut out = Vec::with_capacity(data.len());
        for item in data {
            let emb = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| AppError::new("AI_BAD_RESPONSE", "error.ai.badResponse", "embedding missing"))?;
            out.push(emb.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect());
        }
        Ok(out)
    }

    /// Stream a chat completion. `on_delta(kind, text)` is called per token
    /// chunk. Returns `(usage, truncated)`. A dropped connection before
    /// `[DONE]` is `truncated = true`, not an error (docs/05 §1.4).
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Value,
        params: &Value,
        cancel: &CancellationToken,
        mut on_delta: impl FnMut(&str, &str),
    ) -> AppResult<(Option<TokenUsage>, bool)> {
        let mut body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "stream_options": { "include_usage": true },
        });
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                body[k] = v.clone();
            }
        }

        let resp = retry(|| async {
            self.req(reqwest::Method::POST, "/chat/completions").json(&body).send().await
        })
        .await?;

        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(classify(status.as_u16(), txt));
        }

        let mut stream = resp.bytes_stream().eventsource();
        let mut usage = None;
        let mut truncated = true; // until we see [DONE]

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    truncated = false;
                    break;
                }
                next = stream.next() => {
                    let Some(ev) = next else { break };
                    let ev = match ev {
                        Ok(e) => e,
                        Err(_) => break, // connection dropped -> truncated
                    };
                    if ev.data == "[DONE]" {
                        truncated = false;
                        break;
                    }
                    let Ok(chunk): Result<Value, _> = serde_json::from_str(&ev.data) else { continue };
                    if let Some(u) = chunk.get("usage").filter(|u| !u.is_null()) {
                        usage = Some(TokenUsage {
                            prompt_tokens: u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                            completion_tokens: u.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                        });
                    }
                    if let Some(delta) = chunk.pointer("/choices/0/delta") {
                        if let Some(txt) = delta.get("content").and_then(|c| c.as_str()) {
                            if !txt.is_empty() { on_delta("text", txt); }
                        }
                        // reasoning models
                        for key in ["reasoning_content", "reasoning"] {
                            if let Some(txt) = delta.get(key).and_then(|c| c.as_str()) {
                                if !txt.is_empty() { on_delta("reasoning", txt); }
                            }
                        }
                    }
                }
            }
        }

        Ok((usage, truncated))
    }
}

fn net_err(e: reqwest::Error) -> AppError {
    AppError::new("AI_NETWORK", "error.ai.network", e.to_string()).retriable()
}

/// Map an HTTP status to a retriable/terminal AppError (docs/05 §1.5).
fn classify(status: u16, body: String) -> AppError {
    let snippet: String = body.chars().take(400).collect();
    match status {
        429 | 500 | 502 | 503 | 504 => {
            AppError::new("AI_UPSTREAM", "error.ai.upstream", format!("HTTP {status}: {snippet}")).retriable()
        }
        401 | 403 => AppError::new("AI_AUTH", "error.ai.auth", format!("HTTP {status}: {snippet}")),
        400 | 404 | 422 => AppError::new("AI_REQUEST", "error.ai.request", format!("HTTP {status}: {snippet}")),
        _ => AppError::new("AI_UPSTREAM", "error.ai.upstream", format!("HTTP {status}: {snippet}")),
    }
}

/// Exponential backoff (1s, 2s, 4s ±25% jitter), max 3 attempts, for retriable
/// errors only.
async fn retry<F, Fut>(mut f: F) -> AppResult<reqwest::Response>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    let mut delay_ms = 1000u64;
    for attempt in 0..3 {
        match f().await {
            Ok(resp) => {
                let s = resp.status().as_u16();
                if matches!(s, 429 | 500 | 502 | 503 | 504) && attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(jitter(delay_ms))).await;
                    delay_ms *= 2;
                    continue;
                }
                return Ok(resp);
            }
            Err(e) => {
                if attempt < 2 && (e.is_timeout() || e.is_connect()) {
                    tokio::time::sleep(Duration::from_millis(jitter(delay_ms))).await;
                    delay_ms *= 2;
                    continue;
                }
                return Err(net_err(e));
            }
        }
    }
    Err(AppError::new("AI_UPSTREAM", "error.ai.upstream", "retries exhausted").retriable())
}

fn jitter(ms: u64) -> u64 {
    // ±25% without pulling `rand`
    let span = ms / 4;
    let n = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0) as u64)
        % (span * 2 + 1);
    ms - span + n
}
