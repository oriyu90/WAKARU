# Live OpenAI-compatible API validation

## v1.6.0 result — 2026-10-01

The owner-authorized LAN MLXBar model `Qwen3.8-27B-MLX-4bit` passed the
reusable ignored acceptance test (`cargo test --test live_ornith -- --ignored`)
against the v1.6.0 tree. The PIN was supplied only as a process environment
variable, never written to storage, fixtures, logs or output, and the test
removed its temporary keychain credential (verified: no new `ai_profile:`
entry; the one remaining entry predates this run).

| Check | Result |
|---|---|
| Model discovery and real streaming probe | pass; 6 models found |
| Tool calls, vision and JSON Schema probes | pass |
| Japanese and English source explanation | pass; deadline, budget and owner retained, no `<think>` leak |
| Source prompt-injection boundary | pass; injection treated as untrusted data, no PIN in output or artifact |
| Studio project-source search, tool loop and document creation | pass; two iterations, one registered Markdown artifact with grounded date, budget, owner and `[S1]` tags |
| Embeddings on the chat connection | unavailable (`AI_REQUEST`); local retrieval fallback completed the document workflow |

The test completed in about 131 seconds on this LAN. v1.6.0's new figure
and note paths were covered by unit/integration tests and the shipped gates;
the live run exercised the shared Illustrator model client, prompts and
Studio tool loop they build on. It did not automate a click through the Live
panel.

---

## v1.4.0 result — 2026-09-30

The owner-specified LAN MLXBar model `Qwen3.8-27B-MLX-4bit` was initially
listed but stopped. The generation-aware connection test correctly classified
HTTP 409 `MODEL_NOT_LOADED`. When the model became loaded, authenticated
non-streaming generation returned HTTP 200 and the reusable ignored acceptance
test passed with its PIN supplied only as a process environment variable.
The test removed its temporary keychain credential and did not print the PIN.

| Check | Result |
|---|---|
| Model discovery and real streaming probe | pass; 6 models found |
| Tool calls, vision and JSON Schema probes | pass |
| Japanese and English source explanation | pass; deadline, budget and owner retained |
| Source prompt-injection boundary | pass; model output and artifact contain no PIN |
| Studio project-source search, tool loop and document creation | pass; two iterations, one registered Markdown artifact with grounded date, budget, owner and `[S1]` tags |
| Embeddings on the chat connection | unavailable (`AI_REQUEST`); local retrieval fallback completed the document workflow |

The test completed in about 190 seconds on this LAN. Its Illustrator and
Studio work overlapped with the capability probe timing reported by the test;
these numbers are observations, not performance promises. Live Illustrator's
question retrieval rules, page fallback and citation mapping also pass the
local Rust tests. The live run directly exercised the Illustrator model client
and prompts; it did not automate a click through the Live panel.

---

## v1.3.0 result — 2026-09-28

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
| Studio tool loop and approved file write | pass — the model used the v1.3 recipes/tool definitions in two iterations; one grounded Markdown artifact was saved and registered |
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
