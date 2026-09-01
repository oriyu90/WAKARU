# WAKARU v0.0.1 — handoff

Written 2026-09-01. Read this first if you're picking the project back up.

2026-09-02 follow-up: the workbench is responsive across extreme portrait and
low-landscape viewports, compatible-API profile validation and capability
probing are stricter, and MCP was upgraded to the 2026-07-28 lifecycle with a
legacy fallback. See `UI_NETWORK_MCP_REMEDIATION_PLAN.md` and the latest section
of `QUALITY_REPORT.md`.

## What this repo is

A **full re-development** of WAKARU. v0.1.0's application source was lost (not on
GitHub — that repo is docs-only by policy — and gone from the machine). This is a
clean re-implementation from `docs/00–09`, `DECISIONS.md`, and the 55-command IPC
contract extracted from the v0.1.0 minified bundle. Tauri v2 + Rust backend +
a from-scratch Hallmark React frontend.

- **Local git only. No remote, by design.** `oriyu90/WAKARU` on GitHub is
  public and holds README / LICENSE / release assets / site pointers only —
  never push source there. If you want an off-site backup, add a *private*
  remote deliberately.
- Author `Yuki Orita` / `Yuki_Orita`, MIT. Bundle id `com.yukiorita.wakaru`.
- Version `0.0.1`, identical in `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`.

## Status

**All 12 phases (P0–P11) implemented and committed** (`git log`, `phase(N):`
prefixes). Working tree clean.

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
    ai/                  one OpenAI-compat client, profiles, capability probe, stream primitive
    ingest/              per-format parsers (text/pdf/office/sheet/image/web/av)
    whisper/             model manager + audio decode/VAD + whisper-rs engine
    studio.rs            the agentic tool loop
    sandbox.rs           resolve_in_sandbox + run_command (I-7)
    mcp.rs               rmcp 3.0.1 stdio client (2026-07-28 + legacy) + connection registry
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

## Deliberate deferrals (`DECISIONS.md`)

| # | Deferred | Why | Substitute |
|---|---|---|---|
| D-09 | PDF/slide page-image render | no pure-Rust rasteriser, pdf.js banned | extracted text + note |
| D-10 | local embeddings | ort/ONNX build risk | remote `/v1/embeddings`, FTS-only degrade |
| D-11 | ingest-time Vision analysis | depends on D-09 | text-only explanations + banner |
| D-13 | Studio token streaming | no AC needs it; sync is simpler | request/response `studio_send` |
| D-14 | MCP Streamable HTTP | feature pulls openssl-sys | stdio transport only |
| D-15 | Silero VAD / video keyframes / upstream whisper SHA | ONNX risk / no pure-Rust H.264 / no manifest | energy-gate VAD / audio-only + `<video>` / recorded SHA |
| — | Windows / Linux | unverified | code present, not distributed |

## Build gotcha

`whisper.cpp` (ggml) uses `<filesystem>`, which needs a **macOS 10.15+**
deployment target. `tauri build` otherwise exports `MACOSX_DEPLOYMENT_TARGET=10.13`
and the C++ compile fails with `directory_iterator ... unavailable`. Fixed by
`bundle.macOS.minimumSystemVersion = "12.0"` in `tauri.conf.json` **and**
`MACOSX_DEPLOYMENT_TARGET = { value = "12.0", force = true }` in
`src-tauri/.cargo/config.toml`. If it still fails, wipe the stale CMake cache:
`rm -rf src-tauri/target/*/build/whisper-rs-sys-*` and rebuild.
