//! The protocol-neutral AI client (I-1 — no vendor SDK). Async reqwest on
//! rustls, with OpenAI-compatible and Anthropic-compatible wire adapters.

use crate::domain::ai::{ApiProtocol, TokenUsage};
use crate::error::{AppError, AppResult};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct AiClient {
    http: reqwest::Client,
    base: String,
    protocol: ApiProtocol,
    key: Option<String>,
    extra_headers: Vec<(String, String)>,
}

/// A function/tool call the model asked for during a stream (fragments are
/// reassembled by `index`).
#[derive(Debug, Clone, Default)]
pub struct StreamedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl AiClient {
    pub fn new(
        protocol: ApiProtocol,
        base_url: &str,
        key: Option<String>,
        extra_headers: Vec<(String, String)>,
        timeout_ms: u32,
    ) -> AppResult<Self> {
        // The user configures these endpoints explicitly (often a model server on
        // the LAN or loopback). Honouring the system / `*_PROXY` proxy here just
        // routes those direct requests through a proxy that usually cannot reach a
        // private address — the most common "endpoint unreachable" cause. Opt out.
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(1000) as u64))
            .connect_timeout(Duration::from_secs(15))
            .no_proxy()
            .build()
            .map_err(|e| AppError::internal(format!("http client: {e}")))?;
        Ok(Self {
            http,
            base: ensure_api_version_path(base_url),
            protocol,
            key: key.filter(|k| !k.is_empty()),
            extra_headers,
        })
    }

    /// The configured base URL (already trimmed of a trailing `/`).
    pub fn base_url(&self) -> &str {
        &self.base
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut rb = self.http.request(method, format!("{}{path}", self.base));
        for (h, v) in &self.extra_headers {
            rb = rb.header(h.as_str(), v.as_str());
        }
        if let Some(k) = &self.key {
            rb = match self.protocol {
                ApiProtocol::Openai => rb.bearer_auth(k),
                ApiProtocol::Anthropic => rb.header("x-api-key", k),
            };
        }
        if self.protocol == ApiProtocol::Anthropic {
            rb = rb.header("anthropic-version", "2023-06-01");
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
        let body: Value = resp
            .json()
            .await
            .map_err(|e| classify(status.as_u16(), e.to_string()))?;
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
        if self.protocol == ApiProtocol::Anthropic {
            return Err(AppError::new(
                "AI_UNSUPPORTED",
                "error.ai.unsupported",
                "Anthropic-compatible connections do not expose embeddings",
            ));
        }
        let body = json!({ "model": model, "input": inputs });
        let resp = retry(true, || async {
            self.req(reqwest::Method::POST, "/embeddings")
                .json(&body)
                .send()
                .await
        })
        .await?;
        let status = resp.status();
        let v: Value = resp
            .json()
            .await
            .map_err(|e| classify(status.as_u16(), e.to_string()))?;
        if !status.is_success() {
            return Err(classify(status.as_u16(), v.to_string()));
        }
        let data = v.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
            AppError::new(
                "AI_BAD_RESPONSE",
                "error.ai.badResponse",
                "no data[] in embeddings response",
            )
        })?;
        let mut out = Vec::with_capacity(data.len());
        for item in data {
            let emb = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| {
                    AppError::new(
                        "AI_BAD_RESPONSE",
                        "error.ai.badResponse",
                        "embedding missing",
                    )
                })?;
            out.push(
                emb.iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect(),
            );
        }
        Ok(out)
    }

    /// Stream a chat completion. `on_delta(kind, text)` is called per token
    /// chunk. Returns `(usage, truncated, tool_calls)`. A dropped connection
    /// before `[DONE]` is `truncated = true`, not an error (docs/05 §1.4).
    /// `tool_calls` is populated when the model chooses to call a tool — the
    /// streamed fragments are reassembled by their `index` (docs/05 §4.3,
    /// Studio 10-iteration loop).
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Value,
        params: &Value,
        cancel: &CancellationToken,
        on_delta: impl FnMut(&str, &str),
    ) -> AppResult<(Option<TokenUsage>, bool, Vec<StreamedToolCall>)> {
        match self.protocol {
            ApiProtocol::Openai => {
                self.chat_stream_openai(model, messages, params, cancel, on_delta)
                    .await
            }
            ApiProtocol::Anthropic => {
                self.chat_stream_anthropic(model, messages, params, cancel, on_delta)
                    .await
            }
        }
    }

    async fn chat_stream_openai(
        &self,
        model: &str,
        messages: Value,
        params: &Value,
        cancel: &CancellationToken,
        mut on_delta: impl FnMut(&str, &str),
    ) -> AppResult<(Option<TokenUsage>, bool, Vec<StreamedToolCall>)> {
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

        let resp = retry(false, || async {
            self.req(reqwest::Method::POST, "/chat/completions")
                .json(&body)
                .send()
                .await
        })
        .await?;

        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(classify(status.as_u16(), txt));
        }

        let mut stream = resp.bytes_stream().eventsource();
        let mut usage = None;
        // Some OpenAI-compatible servers for reasoning models emit the chain of
        // thought inline in `content` as a leading `<think>…</think>` block
        // instead of using the separate `reasoning_content` delta field. Route
        // that span to the `reasoning` channel so it never lands in the answer.
        let mut think = ThinkSplit::default();
        let mut truncated = true; // until we see [DONE] or a terminal finish_reason
                                  // OpenAI-compatible providers report an otherwise cleanly terminated
                                  // response as `finish_reason: "length"` when the output budget is
                                  // exhausted. `[DONE]` still follows, so the transport alone cannot
                                  // distinguish this from a complete answer.
        let mut output_limit_reached = false;
        // LM Studio (several configs), llama.cpp's server, and other
        // OpenAI-compatible servers close the SSE stream cleanly right after a
        // `finish_reason` chunk and never send the `[DONE]` sentinel. Seeing a
        // terminal reason means the model finished; only a drop with *no*
        // finish_reason is a real truncation.
        let mut saw_terminal_reason = false;
        // Reassembled by streamed `index`; kept dense so `into_iter` yields
        // calls in their original order.
        let mut tool_calls: Vec<StreamedToolCall> = Vec::new();

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
                        truncated = output_limit_reached;
                        break;
                    }
                    let Ok(chunk): Result<Value, _> = serde_json::from_str(&ev.data) else { continue };
                    match chunk.pointer("/choices/0/finish_reason").and_then(Value::as_str) {
                        Some("length") => {
                            output_limit_reached = true;
                            saw_terminal_reason = true;
                        }
                        Some("stop" | "tool_calls" | "content_filter" | "function_call") => {
                            saw_terminal_reason = true;
                        }
                        _ => {}
                    }
                    if let Some(u) = chunk.get("usage").filter(|u| !u.is_null()) {
                        usage = Some(TokenUsage {
                            prompt_tokens: u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                            completion_tokens: u.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                        });
                    }
                    if let Some(delta) = chunk.pointer("/choices/0/delta") {
                        if let Some(txt) = delta.get("content").and_then(|c| c.as_str()) {
                            if !txt.is_empty() { think.push(txt, &mut on_delta); }
                        }
                        // reasoning models
                        for key in ["reasoning_content", "reasoning"] {
                            if let Some(txt) = delta.get(key).and_then(|c| c.as_str()) {
                                if !txt.is_empty() { on_delta("reasoning", txt); }
                            }
                        }
                        if let Some(calls) = delta.get("tool_calls").and_then(|c| c.as_array()) {
                            for call in calls {
                                let idx = call.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                if tool_calls.len() <= idx {
                                    tool_calls.resize(idx + 1, StreamedToolCall::default());
                                }
                                let slot = &mut tool_calls[idx];
                                if let Some(id) = call.get("id").and_then(|i| i.as_str()) {
                                    if !id.is_empty() { slot.id = id.to_string(); }
                                }
                                if let Some(f) = call.get("function") {
                                    if let Some(name) = f.get("name").and_then(|n| n.as_str()) {
                                        if !name.is_empty() { slot.name.push_str(name); }
                                    }
                                    if let Some(args) = f.get("arguments").and_then(|a| a.as_str()) {
                                        slot.arguments.push_str(args);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        think.finish(&mut on_delta);
        // A clean close after a terminal `finish_reason` is a complete response
        // even on servers that omit `[DONE]`. `length` still counts as
        // output-limit-reached (`output_limit_reached` already set).
        if truncated && saw_terminal_reason {
            truncated = output_limit_reached;
        }
        // Drop any empty slots the model never filled (defensive — sparse index).
        tool_calls.retain(|c| !c.name.is_empty());
        Ok((usage, truncated, tool_calls))
    }

    async fn chat_stream_anthropic(
        &self,
        model: &str,
        messages: Value,
        params: &Value,
        cancel: &CancellationToken,
        mut on_delta: impl FnMut(&str, &str),
    ) -> AppResult<(Option<TokenUsage>, bool, Vec<StreamedToolCall>)> {
        let body = anthropic_body(model, messages, params)?;
        let resp = retry(false, || async {
            self.req(reqwest::Method::POST, "/messages")
                .json(&body)
                .send()
                .await
        })
        .await?;

        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(classify(status.as_u16(), txt));
        }

        let mut stream = resp.bytes_stream().eventsource();
        let mut usage = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
        };
        let mut saw_usage = false;
        let mut truncated = true;
        let mut tool_calls: Vec<StreamedToolCall> = Vec::new();

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
                        Err(_) => break,
                    };
                    let Ok(chunk): Result<Value, _> = serde_json::from_str(&ev.data) else {
                        continue;
                    };
                    match chunk.get("type").and_then(Value::as_str).unwrap_or("") {
                        "message_start" => {
                            if let Some(u) = chunk.pointer("/message/usage") {
                                usage.prompt_tokens = token_count(u, "input_tokens");
                                usage.completion_tokens = token_count(u, "output_tokens");
                                saw_usage = true;
                            }
                        }
                        "content_block_start" => {
                            let idx = chunk.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                            let block = chunk.get("content_block").unwrap_or(&Value::Null);
                            match block.get("type").and_then(Value::as_str).unwrap_or("") {
                                "tool_use" => {
                                    if tool_calls.len() <= idx {
                                        tool_calls.resize(idx + 1, StreamedToolCall::default());
                                    }
                                    tool_calls[idx].id = block.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                                    tool_calls[idx].name = block.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                                    if let Some(input) = block.get("input").filter(|v| v.as_object().is_some_and(|o| !o.is_empty())) {
                                        tool_calls[idx].arguments = input.to_string();
                                    }
                                }
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                                        on_delta("text", text);
                                    }
                                }
                                _ => {}
                            }
                        }
                        "content_block_delta" => {
                            let idx = chunk.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                            let delta = chunk.get("delta").unwrap_or(&Value::Null);
                            match delta.get("type").and_then(Value::as_str).unwrap_or("") {
                                "text_delta" => {
                                    if let Some(text) = delta.get("text").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                                        on_delta("text", text);
                                    }
                                }
                                "thinking_delta" => {
                                    if let Some(text) = delta.get("thinking").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                                        on_delta("reasoning", text);
                                    }
                                }
                                "input_json_delta" => {
                                    if tool_calls.len() <= idx {
                                        tool_calls.resize(idx + 1, StreamedToolCall::default());
                                    }
                                    if let Some(part) = delta.get("partial_json").and_then(Value::as_str) {
                                        tool_calls[idx].arguments.push_str(part);
                                    }
                                }
                                _ => {}
                            }
                        }
                        "message_delta" => {
                            if let Some(u) = chunk.get("usage") {
                                usage.completion_tokens = token_count(u, "output_tokens");
                                saw_usage = true;
                            }
                        }
                        "message_stop" => {
                            truncated = false;
                            break;
                        }
                        "error" => {
                            let message = chunk.pointer("/error/message").and_then(Value::as_str).unwrap_or("Anthropic stream error");
                            return Err(AppError::new("AI_UPSTREAM", "error.ai.upstream", sanitise(message)));
                        }
                        // Ping and future event types are intentionally ignored.
                        _ => {}
                    }
                }
            }
        }

        tool_calls.retain(|call| !call.name.is_empty());
        Ok((saw_usage.then_some(usage), truncated, tool_calls))
    }
}

const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

/// Splits a leading `<think>…</think>` span out of an OpenAI-compatible `content`
/// stream and routes it to the `reasoning` channel. Everything else is passed
/// straight through as `text`, byte for byte. Tags may be split across deltas;
/// the decision is deferred until enough bytes are buffered. A stream that never
/// opens a think block reaches `Passthrough` on its first non-`<` byte, so the
/// common case adds only a one-delta buffer and no data is ever dropped.
enum ThinkSplit {
    /// Not yet known whether the stream opens with a think block.
    Undecided(String),
    /// Inside the think block, buffering until `</think>` is seen.
    Inside(String),
    /// A think block is done, or the stream provably has none.
    Passthrough,
}

impl Default for ThinkSplit {
    fn default() -> Self {
        ThinkSplit::Undecided(String::new())
    }
}

impl ThinkSplit {
    fn push<F: FnMut(&str, &str)>(&mut self, chunk: &str, on: &mut F) {
        match self {
            ThinkSplit::Passthrough => {
                if !chunk.is_empty() {
                    on("text", chunk);
                }
            }
            ThinkSplit::Undecided(buf) => {
                buf.push_str(chunk);
                let trimmed = buf.trim_start();
                if trimmed.is_empty() {
                    return; // only whitespace so far — wait for more
                }
                if let Some(rest) = trimmed.strip_prefix(THINK_OPEN) {
                    let rest = rest.to_string();
                    *self = ThinkSplit::Inside(String::new());
                    self.push(&rest, on);
                } else if THINK_OPEN.starts_with(trimmed) {
                    // `trimmed` is still a proper prefix of "<think>" — wait.
                } else {
                    let out = std::mem::take(buf);
                    *self = ThinkSplit::Passthrough;
                    on("text", &out);
                }
            }
            ThinkSplit::Inside(buf) => {
                buf.push_str(chunk);
                let Some(idx) = buf.find(THINK_CLOSE) else {
                    // Hold the whole block until it closes. If the stream ends
                    // first, `finish` releases it to `text` (nothing lost).
                    return;
                };
                let inner = buf[..idx].to_string();
                let after = buf[idx + THINK_CLOSE.len()..].to_string();
                *self = ThinkSplit::Passthrough;
                if !inner.is_empty() {
                    on("reasoning", &inner);
                }
                if !after.is_empty() {
                    on("text", &after);
                }
            }
        }
    }

    /// Flush whatever is buffered at end of stream. An undecided partial tag or
    /// the body of an unterminated `<think>` block is released to `text` (without
    /// the opening tag) so the content is shown rather than lost.
    fn finish<F: FnMut(&str, &str)>(&mut self, on: &mut F) {
        match std::mem::replace(self, ThinkSplit::Passthrough) {
            ThinkSplit::Undecided(buf) | ThinkSplit::Inside(buf) if !buf.is_empty() => {
                on("text", &buf);
            }
            _ => {}
        }
    }
}

fn anthropic_body(model: &str, messages: Value, params: &Value) -> AppResult<Value> {
    let rows = messages.as_array().ok_or_else(|| {
        AppError::new(
            "AI_REQUEST",
            "error.ai.request",
            "messages must be an array",
        )
    })?;
    let mut system = Vec::new();
    let mut converted = Vec::new();

    for row in rows {
        let role = row.get("role").and_then(Value::as_str).unwrap_or("user");
        if matches!(role, "system" | "developer") {
            let text = content_text(row.get("content").unwrap_or(&Value::Null));
            if !text.is_empty() {
                system.push(text);
            }
            continue;
        }

        let (anthropic_role, mut content) = match role {
            "assistant" => (
                "assistant",
                anthropic_content(row.get("content").unwrap_or(&Value::Null)),
            ),
            "tool" => (
                "user",
                vec![json!({
                    "type": "tool_result",
                    "tool_use_id": row.get("tool_call_id").and_then(Value::as_str).unwrap_or(""),
                    "content": content_text(row.get("content").unwrap_or(&Value::Null)),
                })],
            ),
            _ => (
                "user",
                anthropic_content(row.get("content").unwrap_or(&Value::Null)),
            ),
        };

        if role == "assistant" {
            if let Some(calls) = row.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    let function = call.get("function").unwrap_or(call);
                    let raw = function
                        .get("arguments")
                        .and_then(Value::as_str)
                        .unwrap_or("{}");
                    let input = serde_json::from_str::<Value>(raw)
                        .ok()
                        .filter(Value::is_object)
                        .unwrap_or_else(|| json!({ "raw": raw }));
                    content.push(json!({
                        "type": "tool_use",
                        "id": call.get("id").and_then(Value::as_str).unwrap_or(""),
                        "name": function.get("name").and_then(Value::as_str).unwrap_or(""),
                        "input": input,
                    }));
                }
            }
        }
        if content.is_empty() {
            content.push(json!({ "type": "text", "text": "" }));
        }
        push_anthropic_message(&mut converted, anthropic_role, content);
    }

    let mut body = json!({
        "model": model,
        "messages": converted,
        "max_tokens": 4096,
        "stream": true,
    });
    if !system.is_empty() {
        body["system"] = json!(system.join("\n\n"));
    }

    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            match key.as_str() {
                "model"
                | "messages"
                | "stream"
                | "stream_options"
                | "parallel_tool_calls"
                | "n" => {}
                "response_format" => {
                    if let Some(format) = convert_response_format(value) {
                        body["output_config"] = json!({ "format": format });
                    }
                }
                "max_completion_tokens" => body["max_tokens"] = value.clone(),
                "stop" => body["stop_sequences"] = value.clone(),
                "tools" => body["tools"] = convert_tools(value),
                "tool_choice" => {
                    if let Some(choice) = convert_tool_choice(value) {
                        body["tool_choice"] = choice;
                    }
                }
                _ => body[key] = value.clone(),
            }
        }
    }
    Ok(body)
}

fn anthropic_content(content: &Value) -> Vec<Value> {
    match content {
        Value::String(text) if !text.is_empty() => {
            vec![json!({ "type": "text", "text": text })]
        }
        Value::String(_) => Vec::new(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                Some("text") => Some(json!({
                    "type": "text",
                    "text": part.get("text").and_then(Value::as_str).unwrap_or(""),
                })),
                Some("image_url") => {
                    let url = part
                        .pointer("/image_url/url")
                        .or_else(|| part.get("image_url"))
                        .and_then(Value::as_str)?;
                    Some(anthropic_image(url))
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn anthropic_image(url: &str) -> Value {
    if let Some(rest) = url.strip_prefix("data:") {
        if let Some((meta, data)) = rest.split_once(',') {
            let media_type = meta.split(';').next().unwrap_or("image/png");
            return json!({
                "type": "image",
                "source": { "type": "base64", "media_type": media_type, "data": data },
            });
        }
    }
    json!({ "type": "image", "source": { "type": "url", "url": url } })
}

fn content_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        other if !other.is_null() => other.to_string(),
        _ => String::new(),
    }
}

fn push_anthropic_message(messages: &mut Vec<Value>, role: &str, content: Vec<Value>) {
    if let Some(last) = messages
        .last_mut()
        .filter(|m| m.get("role").and_then(Value::as_str) == Some(role))
    {
        if let Some(parts) = last.get_mut("content").and_then(Value::as_array_mut) {
            parts.extend(content);
            return;
        }
    }
    messages.push(json!({ "role": role, "content": content }));
}

fn convert_tools(tools: &Value) -> Value {
    Value::Array(
        tools
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|tool| {
                let function = tool.get("function").unwrap_or(tool);
                let name = function.get("name")?.as_str()?;
                Some(json!({
                    "name": name,
                    "description": function.get("description").and_then(Value::as_str).unwrap_or(""),
                    "input_schema": function.get("parameters").cloned().unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                }))
            })
            .collect(),
    )
}

fn convert_tool_choice(value: &Value) -> Option<Value> {
    match value.as_str() {
        Some("auto") => Some(json!({ "type": "auto" })),
        Some("required") => Some(json!({ "type": "any" })),
        Some("none") => None,
        _ => value
            .pointer("/function/name")
            .or_else(|| value.get("name"))
            .and_then(Value::as_str)
            .map(|name| json!({ "type": "tool", "name": name })),
    }
}

fn convert_response_format(value: &Value) -> Option<Value> {
    if value.get("type").and_then(Value::as_str) != Some("json_schema") {
        return None;
    }
    let schema = value.pointer("/json_schema/schema")?.clone();
    Some(json!({ "type": "json_schema", "schema": schema }))
}

fn token_count(usage: &Value, field: &str) -> u32 {
    usage
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u32::MAX as u64) as u32
}

/// Complete a base URL that omits the API version segment. Local OpenAI- and
/// Anthropic-compatible servers (LM Studio, Ollama, llama.cpp, vLLM, LocalAI,
/// mlx-bar) — and `api.anthropic.com` itself — serve every route under `/v1`.
/// A base URL of just `scheme://host:port` therefore 404s, or, on LM Studio,
/// draws a bogus `200 "Unexpected endpoint or method"` whose body is not a
/// stream (the reported "no reply" symptom). When the URL carries no path we
/// append `/v1`; any explicit path (`/v1`, `/openai/v1`, a gateway prefix) is
/// left exactly as the user typed it. A string that will not parse is returned
/// unchanged so the caller's own error handling still runs.
pub(crate) fn ensure_api_version_path(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    match reqwest::Url::parse(trimmed) {
        Ok(url) if url.path().is_empty() || url.path() == "/" => format!("{trimmed}/v1"),
        _ => trimmed.to_string(),
    }
}

fn net_err(e: reqwest::Error) -> AppError {
    AppError::new("AI_NETWORK", "error.ai.network", e.to_string()).retriable()
}

/// Map an HTTP status to a retriable/terminal AppError (docs/05 §1.5).
fn classify(status: u16, body: String) -> AppError {
    let snippet = sanitise(&body);
    match status {
        429 | 500 | 502 | 503 | 504 | 529 => AppError::new(
            "AI_UPSTREAM",
            "error.ai.upstream",
            format!("HTTP {status}: {snippet}"),
        )
        .retriable(),
        401 | 403 => AppError::new(
            "AI_AUTH",
            "error.ai.auth",
            format!("HTTP {status}: {snippet}"),
        ),
        400 | 404 | 422 => AppError::new(
            "AI_REQUEST",
            "error.ai.request",
            format!("HTTP {status}: {snippet}"),
        ),
        _ => AppError::new(
            "AI_UPSTREAM",
            "error.ai.upstream",
            format!("HTTP {status}: {snippet}"),
        ),
    }
}

/// Exponential backoff (1s, 2s, 4s ±25% jitter), max 3 attempts.
///
/// `retry_5xx` retries a `429/5xx` *response*; pass `false` for a non-idempotent
/// request (a streaming chat completion) where a retry after the server already
/// accepted the call would trigger a second full generation. Connect/timeout
/// errors — where no request reached the model — are always retried.
async fn retry<F, Fut>(retry_5xx: bool, mut f: F) -> AppResult<reqwest::Response>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    let mut delay_ms = 1000u64;
    for attempt in 0..3 {
        match f().await {
            Ok(resp) => {
                let s = resp.status().as_u16();
                if retry_5xx && matches!(s, 429 | 500 | 502 | 503 | 504 | 529) && attempt < 2 {
                    let wait = retry_after(&resp)
                        .unwrap_or_else(|| Duration::from_millis(jitter(delay_ms)));
                    tokio::time::sleep(wait).await;
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

fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    let seconds = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

/// Upstream bodies can echo request metadata. Expose only the provider's error
/// message when possible and cap plain-text responses before they reach logs/UI.
fn sanitise(body: &str) -> String {
    let safe = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.to_string());
    safe.chars().take(400).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    #[test]
    fn anthropic_adapter_moves_system_tools_and_results_to_native_shape() {
        let body = anthropic_body(
            "claude-test",
            json!([
                { "role": "system", "content": "Be precise" },
                { "role": "user", "content": "Find it" },
                {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": "tool-1",
                        "type": "function",
                        "function": { "name": "search", "arguments": "{\"q\":\"x\"}" }
                    }]
                },
                { "role": "tool", "tool_call_id": "tool-1", "content": "found" }
            ]),
            &json!({
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": "search",
                        "description": "Search",
                        "parameters": { "type": "object", "properties": { "q": { "type": "string" } } }
                    }
                }],
                "tool_choice": "required",
                "response_format": {
                    "type": "json_schema",
                    "json_schema": { "name": "answer", "schema": { "type": "object" } }
                }
            }),
        )
        .unwrap();

        assert_eq!(body["system"], "Be precise");
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(body["tool_choice"]["type"], "any");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_use");
        assert_eq!(body["messages"][2]["content"][0]["type"], "tool_result");
    }

    #[tokio::test]
    async fn anthropic_stream_restores_text_tool_calls_usage_and_headers() {
        let events = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":7,\"output_tokens\":1}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"lookup\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"id\\\":1}\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":4}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );
        let (base, request_rx) = serve_once(events);
        let client = AiClient::new(
            ApiProtocol::Anthropic,
            &base,
            Some("secret-test-key".into()),
            Vec::new(),
            5_000,
        )
        .unwrap();
        let mut text = String::new();
        let (usage, truncated, calls) = client
            .chat_stream(
                "claude-test",
                json!([{ "role": "system", "content": "sys" }, { "role": "user", "content": "hi" }]),
                &json!({ "max_tokens": 10 }),
                &CancellationToken::new(),
                |kind, delta| {
                    if kind == "text" {
                        text.push_str(delta);
                    }
                },
            )
            .await
            .unwrap();

        assert_eq!(text, "Hi");
        assert!(!truncated);
        assert_eq!(usage.unwrap().completion_tokens, 4);
        assert_eq!(calls[0].name, "lookup");
        assert_eq!(calls[0].arguments, "{\"id\":1}");

        let request = request_rx.recv().unwrap();
        let lower = request.to_ascii_lowercase();
        // The mock base URL has no path, so the client completes it to `/v1`.
        assert!(request.starts_with("POST /v1/messages HTTP/1.1"));
        assert!(lower.contains("x-api-key: secret-test-key"));
        assert!(lower.contains("anthropic-version: 2023-06-01"));
        assert!(request.contains("\"system\":\"sys\""));
    }

    fn run_think_split(chunks: &[&str]) -> (String, String) {
        let mut split = ThinkSplit::default();
        let mut text = String::new();
        let mut reasoning = String::new();
        let mut sink = |kind: &str, delta: &str| match kind {
            "reasoning" => reasoning.push_str(delta),
            _ => text.push_str(delta),
        };
        for chunk in chunks {
            split.push(chunk, &mut sink);
        }
        split.finish(&mut sink);
        (text, reasoning)
    }

    #[test]
    fn think_split_routes_a_one_delta_block_to_reasoning() {
        let (text, reasoning) =
            run_think_split(&["<think>weighing options</think>The answer is 42."]);
        assert_eq!(text, "The answer is 42.");
        assert_eq!(reasoning, "weighing options");
    }

    #[test]
    fn think_split_reassembles_tags_split_across_deltas() {
        let (text, reasoning) = run_think_split(&[
            "<",
            "think>",
            "step one",
            " step two",
            "</",
            "think",
            ">",
            "Done",
            ".",
        ]);
        assert_eq!(text, "Done.");
        assert_eq!(reasoning, "step one step two");
    }

    #[test]
    fn think_split_leaves_ordinary_content_untouched() {
        let (text, reasoning) = run_think_split(&["The <thing> ", "is <not> a think block."]);
        assert_eq!(text, "The <thing> is <not> a think block.");
        assert!(reasoning.is_empty());
    }

    #[test]
    fn think_split_handles_leading_whitespace_before_the_tag() {
        let (text, reasoning) = run_think_split(&["\n  <think>hmm</think>\nA."]);
        assert_eq!(text, "\nA.");
        assert_eq!(reasoning, "hmm");
    }

    #[test]
    fn think_split_flushes_an_unterminated_block_to_text_without_loss() {
        let (text, reasoning) = run_think_split(&["<think>still thinking when the stream died"]);
        assert_eq!(text, "still thinking when the stream died");
        assert!(reasoning.is_empty());
    }

    #[test]
    fn think_split_passes_through_when_stream_never_opens_a_block() {
        let (text, reasoning) = run_think_split(&["plain answer, no tags at all"]);
        assert_eq!(text, "plain answer, no tags at all");
        assert!(reasoning.is_empty());
    }

    #[tokio::test]
    async fn openai_stream_splits_inline_think_block_from_the_answer() {
        let events = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"<think>I should\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" be careful</think>Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" there\"},\"finish_reason\":null}]}\n\n",
            "data: [DONE]\n\n"
        );
        let (base, _rx) = serve_once(events);
        let client = AiClient::new(ApiProtocol::Openai, &base, None, Vec::new(), 5_000).unwrap();
        let mut text = String::new();
        let mut reasoning = String::new();
        let (_usage, truncated, calls) = client
            .chat_stream(
                "test-model",
                json!([{ "role": "user", "content": "hi" }]),
                &json!({ "max_tokens": 32 }),
                &CancellationToken::new(),
                |kind, delta| match kind {
                    "reasoning" => reasoning.push_str(delta),
                    _ => text.push_str(delta),
                },
            )
            .await
            .unwrap();
        assert_eq!(text, "Hello there");
        assert_eq!(reasoning, "I should be careful");
        assert!(!truncated);
        assert!(calls.is_empty());
    }

    #[tokio::test]
    async fn openai_length_finish_is_reported_as_truncated_even_with_done() {
        let events = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n",
            "data: [DONE]\n\n"
        );
        let (base, _request_rx) = serve_once(events);
        let client = AiClient::new(ApiProtocol::Openai, &base, None, Vec::new(), 5_000).unwrap();
        let mut text = String::new();
        let (_usage, truncated, calls) = client
            .chat_stream(
                "test-model",
                json!([{ "role": "user", "content": "write" }]),
                &json!({ "max_tokens": 3 }),
                &CancellationToken::new(),
                |kind, delta| {
                    if kind == "text" {
                        text.push_str(delta);
                    }
                },
            )
            .await
            .unwrap();

        assert_eq!(text, "partial");
        assert!(truncated);
        assert!(calls.is_empty());
    }

    // LM Studio / llama.cpp and other OpenAI-compatible servers close the stream
    // cleanly after a `finish_reason` chunk and never send `data: [DONE]`.
    #[tokio::test]
    async fn openai_stream_completes_without_done_after_stop_finish_reason() {
        let events = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n"
        );
        let (base, _rx) = serve_once(events);
        let client = AiClient::new(ApiProtocol::Openai, &base, None, Vec::new(), 5_000).unwrap();
        let mut text = String::new();
        let (_usage, truncated, calls) = client
            .chat_stream(
                "test-model",
                json!([{ "role": "user", "content": "hi" }]),
                &json!({}),
                &CancellationToken::new(),
                |kind, delta| {
                    if kind == "text" {
                        text.push_str(delta);
                    }
                },
            )
            .await
            .unwrap();
        assert_eq!(text, "Hello world");
        assert!(
            !truncated,
            "a clean close after finish_reason:stop is complete"
        );
        assert!(calls.is_empty());
    }

    #[tokio::test]
    async fn mlxbar_stream_contract_handles_keepalive_reasoning_usage_and_done() {
        let events = concat!(
            ": mlxbar keep-alive\n\n",
            "data: {\"id\":\"chatcmpl-test\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"reasoning_content\":\"checking\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Grounded answer\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-test\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"id\":\"chatcmpl-test\",\"choices\":[],\"usage\":{\"prompt_tokens\":23,\"completion_tokens\":4,\"total_tokens\":27}}\n\n",
            "data: [DONE]\n\n"
        );
        let (base, request_rx) = serve_once(events);
        let client = AiClient::new(
            ApiProtocol::Openai,
            &base,
            Some("mlxbar-token".into()),
            Vec::new(),
            5_000,
        )
        .unwrap();
        let mut answer = String::new();
        let mut reasoning = String::new();
        let (usage, truncated, calls) = client
            .chat_stream(
                "Qwen3.5-9B-MLX",
                json!([{ "role": "user", "content": "question" }]),
                &json!({ "max_tokens": 128 }),
                &CancellationToken::new(),
                |kind, delta| match kind {
                    "reasoning" => reasoning.push_str(delta),
                    _ => answer.push_str(delta),
                },
            )
            .await
            .unwrap();

        assert_eq!(answer, "Grounded answer");
        assert_eq!(reasoning, "checking");
        assert_eq!(usage.unwrap().prompt_tokens, 23);
        assert!(!truncated);
        assert!(calls.is_empty());
        let request = request_rx.recv().unwrap();
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer mlxbar-token"));
        assert!(request.contains("\"stream_options\":{\"include_usage\":true}"));
    }

    #[tokio::test]
    async fn mlxbar_model_descriptors_are_accepted_without_guessing_from_metadata() {
        let body = r#"{"object":"list","data":[{"id":"Qwen3.5-9B-MLX","object":"model","owned_by":"mlxbar","loaded":true,"max_tokens":8192,"context_window":32768,"modalities":["text"]},{"id":"VLM/model","object":"model","owned_by":"mlxbar","loaded":false,"modalities":["text","image"]}]}"#;
        let (base, request_rx) = serve_once(body);
        let client = AiClient::new(ApiProtocol::Openai, &base, None, Vec::new(), 5_000).unwrap();
        assert_eq!(
            client.list_models().await.unwrap(),
            vec!["Qwen3.5-9B-MLX", "VLM/model"]
        );
        assert!(request_rx
            .recv()
            .unwrap()
            .starts_with("GET /v1/models HTTP/1.1"));
    }

    // A real mid-stream drop (no finish_reason, no [DONE]) is still truncation.
    #[tokio::test]
    async fn openai_stream_dropped_without_finish_reason_is_truncated() {
        let events =
            "data: {\"choices\":[{\"delta\":{\"content\":\"half a sen\"},\"finish_reason\":null}]}\n\n";
        let (base, _rx) = serve_once(events);
        let client = AiClient::new(ApiProtocol::Openai, &base, None, Vec::new(), 5_000).unwrap();
        let mut text = String::new();
        let (_usage, truncated, _calls) = client
            .chat_stream(
                "test-model",
                json!([{ "role": "user", "content": "hi" }]),
                &json!({}),
                &CancellationToken::new(),
                |_k, d| text.push_str(d),
            )
            .await
            .unwrap();
        assert_eq!(text, "half a sen");
        assert!(truncated);
    }

    #[test]
    fn ensure_api_version_path_appends_v1_only_when_the_url_has_no_path() {
        for (input, want) in [
            ("http://localhost:1234", "http://localhost:1234/v1"),
            ("http://localhost:1234/", "http://localhost:1234/v1"),
            (
                "  http://192.168.0.114:1234/  ",
                "http://192.168.0.114:1234/v1",
            ),
            ("https://api.anthropic.com", "https://api.anthropic.com/v1"),
            ("http://localhost:1234/v1", "http://localhost:1234/v1"),
            ("https://api.openai.com/v1", "https://api.openai.com/v1"),
            (
                "https://gw.example.com/openai/v1",
                "https://gw.example.com/openai/v1",
            ),
            ("not a url", "not a url"),
        ] {
            assert_eq!(ensure_api_version_path(input), want, "input {input:?}");
        }
    }

    // The user's LM Studio profile was `http://host:1234` with no `/v1`; the
    // request must still land on `/v1/chat/completions`, not `/chat/completions`.
    #[tokio::test]
    async fn openai_client_targets_v1_when_the_base_url_omits_it() {
        let events =
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\n";
        let (base, request_rx) = serve_once(events);
        let client = AiClient::new(ApiProtocol::Openai, &base, None, Vec::new(), 5_000).unwrap();
        let _ = client
            .chat_stream(
                "m",
                json!([{ "role": "user", "content": "hi" }]),
                &json!({}),
                &CancellationToken::new(),
                |_, _| {},
            )
            .await
            .unwrap();
        let request = request_rx.recv().unwrap();
        assert!(
            request.starts_with("POST /v1/chat/completions HTTP/1.1"),
            "request line was {:?}",
            request.lines().next().unwrap_or_default()
        );
    }

    fn serve_once(body: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = socket.read(&mut buffer).unwrap_or(0);
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request_complete(&request) {
                    break;
                }
            }
            tx.send(String::from_utf8_lossy(&request).to_string())
                .unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}"), rx)
    }

    fn request_complete(request: &[u8]) -> bool {
        let text = String::from_utf8_lossy(request);
        let Some((headers, body)) = text.split_once("\r\n\r\n") else {
            return false;
        };
        let length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_string)
            })
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        body.len() >= length
    }
}
