# WAKARU handoff

Written 2026-09-01. Read this first if you're picking the project back up.

**2026-09-03 · v0.0.4** — OCR for scanned PDFs and images (`ocrs`+`rten`, pure
Rust, models downloaded on first use; `commands/ocr.rs`, `services/ocr.rs`,
`migrations/project/003_ocr.sql` → `sources.ocr_status`), a `build_document`
Studio tool (Markdown/DOCX/PDF; `services/doc_builder.rs` + `services/pdf_text.rs`
using a **system** CJK font via `fontdb`, no font bundled), and an `ocr.enabled`
setting. See `IMPLEMENTATION_PLAN_v0.0.4.md` and `docs/DECISIONS.md` D-22–D-25.
**The repository is PRIVATE for this cycle** — Release assets are collaborator-only.

**2026-09-03 · v0.0.3** — LAN AI connection fix (`.no_proxy()` on the AI client,
`NSLocalNetworkUsageDescription` in `src-tauri/Info.plist`, real connection-test
diagnostics), responsive/centred layout, dark-theme contrast, and a macOS-native
visual register (San Francisco face, overlay title bar / unified toolbar, macOS
control geometry). **The application source is now public at `oriyu90/WAKARU`
(MIT) with full history** — the earlier "docs-only / never push source" policy
was reversed by the owner (`DECISIONS.md` D-21). Commit author was rewritten to
`Yuki Orita <yukiorita0911.official@gmail.com>` across history. See
`IMPLEMENTATION_PLAN_v0.0.3.md` §8 and `QUALITY_REPORT.md`.

2026-09-02 v0.0.2: original files render in the Viewer, local semantic embeddings
and image Vision augmentation, Studio live text/tool progress, policy-checked
Streamable HTTP MCP, Silero VAD with a deterministic fallback. See
`IMPLEMENTATION_PLAN_v0.0.2.md`.

## What this repo is

A **full re-development** of WAKARU. v0.1.0's application source was lost (the
GitHub repo held only docs at the time, and the working tree was gone from the
machine). This is a clean re-implementation from `docs/00–09`, `DECISIONS.md`,
and the 55-command IPC contract extracted from the v0.1.0 minified bundle.
Tauri v2 + Rust backend + a from-scratch Hallmark React frontend. Since v0.0.3
the source is public on `oriyu90/WAKARU`.

- **`oriyu90/WAKARU` on GitHub is now the public source repository** (MIT, full
  history). `main` is authoritative. Releases carry the macOS DMG + `.sha256`.
- Author `Yuki_Orita` <yukiorita0911.official@gmail.com>, MIT. Bundle id
  `com.yukiorita.wakaru`.
- Version must match in `package.json` / `package-lock.json` /
  `src-tauri/Cargo.toml` / `src-tauri/Cargo.lock` / `src-tauri/tauri.conf.json`.

## Status

All original phases are implemented. The v0.0.2 replacement is finalized only
after the quality gates and release record in `QUALITY_REPORT.md` are green.

| Gate | How | State |
|---|---|---|
| Frontend | `npm run typecheck && npm run lint && npm test && npm run check:contrast && npm run check:i18n` | green — 9 tests |
| Backend | `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-targets --all-features` | green — 185 tests; live Ornith and manual MCP tests ignored by default; manual MCP test passes |
| Licenses | `cd src-tauri && cargo deny check licenses bans sources` | green — no GPL/AGPL/LGPL |

The current release work follows `IMPLEMENTATION_PLAN_v0.0.0.md`: dual API
formats, built-in Studio/Illustrator behaviour, reliability hardening, final
DMG verification, then public docs and Studio RIZI updates.

The 2026-09-01 replacement at source commit `0c3d411` additionally separates
all document/RAG context from system instructions, strengthens Studio file
creation for smaller models, blocks suspiciously shortened Markdown output,
and uses exact-white primary text in dark mode. See `STATIC_FUNCTION_AUDIT.md`.
The owner-authorised Ornith live endpoint validation and the output-limit bug
found through it are recorded in `LIVE_ORNITH_VALIDATION.md`.

## Layout

```
src/                     React frontend (Hallmark). features/, components/, ipc/, i18n/, stores/, styles/
src-tauri/src/
  commands/              thin #[tauri::command] wrappers only
  domain/                ts-rs types crossing the IPC boundary → src/ipc/types.gen.ts
  services/              all business logic
    ai/                  OpenAI/Anthropic-compatible clients, embeddings, capability probes
    ingest/              per-format parsers (text/pdf/office/sheet/image/web/av)
    whisper/             model manager + audio decode/VAD + whisper-rs engine
    studio.rs            the agentic tool loop
    sandbox.rs           resolve_in_sandbox + run_command (I-7)
    mcp.rs               rmcp 3.0.1 stdio/Streamable HTTP client + connection registry
  storage/               rusqlite, migrations, forward-only runner
  jobs/                  tokio worker pool + job://progress
migrations/{app,project}/*.sql
docs/                    00–09 spec, DECISIONS.md, ipc-contract.md, this file
scripts/                 check-{design-rules,contrast,i18n,hardcoded}.mjs
```

## Conventions that matter

- **Never hold a `rusqlite::Connection` across `.await`** — it isn't `Send`.
  Pattern: sync-resolve config → drop conn → await → reopen. Every async
  service fn does this.
- ts-rs types export to `src/ipc/types.gen.ts` via `cargo test export_bindings`;
  CI would fail on a diff. `TS_RS_EXPORT_DIR` is pinned in `.cargo/config.toml`.
- Frontend colours/space/type come only from `src/styles/tokens.css`;
  `check-design-rules.mjs` enforces it. `check-hardcoded.mjs` blocks English
  JSX text not wrapped in `t()`. i18n keys must match across en/ja/zh-Hans.
- Phase gate before each `phase(N):` commit = the full gate table above.

## v0.0.2 resolution of former deferrals (`DECISIONS.md`, D-17)

| # | v0.0.2 state | Implementation |
|---|---|---|
| D-09 | completed | bundled PDF.js, DOCX/PPTX browser renderers and spreadsheet tables |
| D-10 | completed | fastembed multilingual-e5-small, cached locally; FTS fallback on failure |
| D-11 | completed for image sources | structured Vision OCR/layout/diagram augmentation; PDF text and visual rendering remain separate |
| D-13 | completed | tab-scoped text delta and tool-state events with cancellation safety |
| D-14 | completed | stdio + rustls Streamable HTTP with private-LAN plaintext policy |
| D-15 | completed | embedded Silero VAD with energy fallback; bounded runtime video thumbnails |
| — | externally unverified | Windows/Linux and Apple notarization are not distributed in v0.0.2 |

## Build gotcha

`whisper.cpp` (ggml) uses `<filesystem>`, which needs a **macOS 10.15+**
deployment target. `tauri build` otherwise exports `MACOSX_DEPLOYMENT_TARGET=10.13`
and the C++ compile fails with `directory_iterator ... unavailable`. Fixed by
`bundle.macOS.minimumSystemVersion = "12.0"` in `tauri.conf.json` **and**
`MACOSX_DEPLOYMENT_TARGET = { value = "12.0", force = true }` in
`src-tauri/.cargo/config.toml`. If it still fails, wipe the stale CMake cache:
`rm -rf src-tauri/target/*/build/whisper-rs-sys-*` and rebuild.
