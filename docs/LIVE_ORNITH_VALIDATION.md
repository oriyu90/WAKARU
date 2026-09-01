# Ornith live API validation — 2026-09-01

## Scope

WAKARU's real OpenAI-compatible Rust client and complete Studio tool loop were
tested against `Ornith-1.5-35B-A3B-MLX-4bit` on the owner-supplied LAN endpoint.
The API credential was supplied only through a process environment variable,
was never printed, and the temporary keychain entry was deleted by the test.

The reusable test is `src-tauri/tests/live_ornith.rs`. It is ignored during the
normal suite because it requires an explicitly authorised live endpoint:

```bash
WAKARU_LIVE_API_KEY=... cargo test --test live_ornith \
  ornith_live_wakaru_path -- --ignored --nocapture
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
| Live Illustrator teaching behaviour | pass — plain explanation, analogy, pitfall, one understanding check |
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
