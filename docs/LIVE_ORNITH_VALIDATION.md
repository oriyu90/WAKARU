# Live OpenAI-compatible API validation

## v0.1.0 result — 2026-09-08

The reusable live acceptance test completed against the owner-authorized local
endpoint with `Qwen3.8-27B-MLX-4bit`. The credential was supplied only as a
process environment variable, was never written to WAKARU storage or output,
and the test removed its temporary keychain credential.

| Check | Result |
|---|---|
| Authenticated model discovery and streaming | pass — 6 models discovered |
| Tool capability probe | pass — the bounded probe now permits local-model tool-call prelude tokens |
| Vision and JSON Schema probes | pass |
| Live Illustrator source grounding | pass — Japanese and English explanations retained date, budget, and owner facts |
| Prompt-injection boundary | pass — untrusted source text remained data and no credential was disclosed |
| Studio tool loop and approved file write | pass — one Markdown artifact was saved and registered |
| Embeddings on the chat connection | unavailable — the endpoint returned a handled request error; WAKARU safely retains text-index search |
| Secret leakage | none in test output, model output, or generated artifact |

The complete ignored test is run only with an explicitly authorized endpoint:

```bash
WAKARU_LIVE_API_KEY=... \\
WAKARU_LIVE_BASE_URL=http://host.example/v1 \\
WAKARU_LIVE_MODEL=example-chat-model \\
WAKARU_LIVE_EMBEDDING_MODEL=example-embedding-model \\
cargo test --test live_ornith \\
  live_openai_wakaru_path -- --ignored --nocapture
```

---

# Earlier live API validation — 2026-09-01

## Scope

WAKARU's real OpenAI-compatible Rust client and complete Studio tool loop were
tested against `Ornith-1.5-35B-A3B-MLX-4bit` on the owner-supplied LAN endpoint.
The API credential was supplied only through a process environment variable,
was never printed, and the temporary keychain entry was deleted by the test.

The reusable test is `src-tauri/tests/live_ornith.rs`. It is ignored during the
normal suite because it requires an explicitly authorised live endpoint. Its
endpoint, chat model, and embedding model are environment settings, so the test
does not turn one LAN server or model family into an application default:

```bash
WAKARU_LIVE_API_KEY=... \
WAKARU_LIVE_BASE_URL=http://host.example/v1 \
WAKARU_LIVE_MODEL=example-chat-model \
WAKARU_LIVE_EMBEDDING_MODEL=example-embedding-model \
cargo test --test live_ornith \
  live_openai_wakaru_path -- --ignored --nocapture
```

## Result

| Check | Result |
|---|---|
| `/v1/models` and authenticated OpenAI-compatible chat | pass |
| Model discovery | pass — requested model present among 15 models |
| Streaming through WAKARU `AiClient` | pass |
| Vision capability probe | pass |
| Tool-calling capability probe | pass |
| JSON Schema capability probe | unsupported by this endpoint/model |
| `/v1/embeddings` | unsupported (HTTP 404); WAKARU correctly remains FTS-only |
| Live Illustrator teaching behaviour | pass — Japanese and English prompts produce a plain explanation and one understanding check |
| Source grounding | pass — deadline, budget and owner preserved |
| Prompt-injection boundary | pass — malicious source sentence treated as data; credential not disclosed |
| Studio source retrieval and tool loop | pass — four model round trips |
| Studio document creation | pass — `workspace/kestrel-summary.md` created and registered as one artifact |
| Secret leakage | none in model output, artifact, or test output |

Observed successful-run timings on the LAN were approximately 23 seconds for
the full capability probe, 22 seconds for the detailed Illustrator response,
and 7 seconds for the four-round Studio retrieval/write workflow. These are
single-run observations, not performance guarantees.

## Defect found and fixed

The endpoint correctly emits `finish_reason: "length"` followed by `[DONE]`
when a generation exhausts `max_tokens`. WAKARU previously treated every
`[DONE]` as complete, so a sentence cut off at the output limit could be shown
as a successful answer. The OpenAI-compatible stream parser now preserves that
provider signal and returns `truncated = true`; Illustrator avoids caching the
partial response and Studio returns the existing retriable truncation error.
A local SSE regression test covers this exact sequence.

## Interpretation

This model is usable for WAKARU's chat, Live Illustrator, and Studio document
work. The built-in ELI5/Socratic/editorial instructions materially affect its
output, and the smaller-model write-file guidance succeeds. For semantic
search, configure a separate endpoint that actually implements
`/v1/embeddings`; merely listing an embedding model is insufficient.
