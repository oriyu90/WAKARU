# WAKARU handoff

Written 2026-09-01. Read this first if you're picking the project back up.

**2026-09-10 · v0.2.0 round 2 (same tag)** — seven more reader issues + a wire
re-audit. (1) **LM Studio "Unexpected endpoint / no reply"**: the profile base
URL had no `/v1`. `ensure_api_version_path()` (`src-tauri/src/services/ai/client.rs`)
appends `/v1` only when the URL has no path; used in `AiClient::new` (covers
probe/chat/embeddings + all stored profiles, no migration) and
`profiles::normalise_base_url`. (2) **Past Studio conversations shown before
opening**: `Studio.tsx` drops the `?? rows[0]` fallback + pin-to-first effect;
unselected shows an `EmptyState`, history isn't fetched until a tab is clicked.
(3) **Live Illustrator explains the whole source on open**: `illustrator_generate`
handles `locator {t:"whole"}` → bounded whole-source digest (≤8 000 chars) + a
prompt prefix re-framing "this page" as "this document"; the drawer defaults to
the 資料全体 view with a このページ segment. (4) **Illustrator ⟂ Studio**: the
"→ Studio" button + `importToStudio` call removed from the panel (backend command
kept); phase6 asserts `list_tabs` never returns an illustrator thread. (5)
**Panel rebuilt** (`IllustratorDrawer.tsx` + css): one header block, scope/detail
segments, explanation as hero, Q&A collapsed, one ask row; `Tabs` gains
`disabled` items. (6) **Auto-show**: `Viewer` opens the drawer when Illustrator
is on and a document tab is active; enabling from off opens it but waits for one
"解説をはじめる" tap (`autoRun={!justEnabled}`). (7) **Model list**: new
`ai_list_models(profileId)` command; `AiSettings` role rows offer a `<Select>` of
discovered model ids + a "type it in" escape + `↻`, and the connection editor
gets a `datalist` + fetch button. No ts-rs change. See `docs/DECISIONS.md` D-32.

**2026-09-09 · v0.2.0 feature + reliability** — seven reader issues + a wire
audit + dangerous-design items. (1) **LM Studio replies not returning**:
`chat_stream_openai` marked a stream `truncated` unless it saw `data: [DONE]`;
now a terminal `finish_reason` counts as complete without it
(`src-tauri/src/services/ai/client.rs`, 2 tests). (2) **Studio + tool-less
models**: `run_loop` (now via `stream_round`) retries a `400` that carried
`tools` once without them and keeps them off for that run
(`src-tauri/src/services/studio.rs`). (3) `retry()` gained `retry_5xx` — the
streaming chat POST no longer re-sends on 5xx (double-generation). (4) **Studio
tab loading**: `list_tabs` is metadata + `COUNT(*)`; new `studio_get_tab(id)`
returns one conversation; `Studio.tsx` fetches the active tab's history
separately and pins `activeId`. `StudioTab` gained `messageCount` (ts-rs diff).
(5) **Viewer redesign**: vertical tab rail (home · open docs · 追加 / リンク /
ライブ解説 at the foot) replacing the horizontal strip + the invisible
`.handle`; add-files / add-URL + dialog lifted from `SourceListPanel` into
`Viewer`. (6) **Live Illustrator toggle**: the rail button enables+opens when
off, opens/closes when on. (7) **PDF fit-to-width**: `ResizeObserver` in
`FilePreviews.tsx`; scale = fit × userZoom × raster. (8) **Fullscreen chrome**:
`main.tsx` tracks `onResized` → `<html data-fullscreen>`; `base.css` drops the
traffic-light inset there; `tauri.conf.json` sets `trafficLightPosition`. (9)
`useUiStore.subscribe` only re-applies display prefs on a real change. Composer
clears in `onMutate`. No project-format change, no migration; v0.0.0–v0.1.3
data opens as-is. See `IMPLEMENTATION_PLAN_v0.2.0.md` and `docs/DECISIONS.md`
D-31.

**2026-09-09 · v0.1.3 maintenance** — five reader-reported issues, all fixed
frontend-only (no IPC / schema / type / routing change; ts-rs bindings identical;
no migration). (1) The Settings switches were still click-dead in the packaged
WKWebView build — v0.1.2's `pointer-events:none` only covered Chromium. `Switch`
is now a `<label>` so any press on it reaches the control in every engine
(`src/components/Switch.tsx`). (2) New `src/components/ContextMenu.tsx` (portalled
into `#app`, `position:absolute` for the monochrome filter, roving focus, closes
on Escape/outside/scroll); wired to the sidebar project links in `AppShell` for
Export (ZIP) / Delete. (3) The top-bar Settings button is a toggle — press again
to return to the previous view (`AppShell` keeps the last non-`/settings` path).
(4) `Viewer.module.css` `.pane > * { flex:1; min-width:0 }` — the "資料を見る"
pane was collapsing to its content width and leaving the right half blank. (5)
Live Illustrator Q&A threads are now per-source, not per-page-locator, so a
conversation survives a page turn (`IllustratorDrawer` query key + stable
`{t:"whole"}` locator; `illustrator_ask` already ignored the thread locator, so no
Rust change); page explanations stay per-page in `illustrations`. Studio pins the
active tab across refetches and no longer double-renders a streamed reply. Backend
persistence was audited and found correct. See `IMPLEMENTATION_PLAN_v0.1.3.md` and
`docs/DECISIONS.md` D-30.

**2026-09-09 · v0.1.2 maintenance** — Settings switches (`Switch` component) were
click-dead: the decorative track/thumb spans painted over the visually hidden
`<input>` with no `pointer-events: none`, so only keyboard toggled them — reported
as "Live Illustrator can't be enabled". Fixed in `src/components/controls.module.css`
(+ `src/components/Switch.test.tsx`). stdio MCP command resolution widened: a bare
`npx` / `uvx` / `node` is now found in the usual install locations (Homebrew,
`~/.local/bin`, cargo/bun/deno/volta, nvm/fnm) even when the app is launched from
Finder with a minimal `PATH`, and the child gets that same widened `PATH`
(`extra_bin_dirs` / `child_path` / `resolve_program` in `src-tauri/src/services/mcp.rs`).
A "SearXNG (web search)" preset in MCP settings pre-fills a known-good stdio config.
No IPC / schema / type / routing change; ts-rs bindings unchanged. See
`IMPLEMENTATION_PLAN_v0.1.2.md` and `docs/DECISIONS.md` D-29.

**2026-09-09 · v0.1.1 maintenance** — `AiClient` OpenAI-compatible streaming now
splits a leading inline `<think>…</think>` block out of `content` and sends it to
the `reasoning` channel (`ThinkSplit` in `src-tauri/src/services/ai/client.rs`),
so a reasoning model that does not use the `reasoning_content` delta cannot leak
its chain of thought into Live Illustrator / Studio / the organizer. Live
Illustrator clears a pending question error when a new explanation starts.
`src-tauri/deny.toml` records `RUSTSEC-2024-0436` (`paste`, maintenance-status,
transitive via `fastembed`) as an accepted exception. No backend contract, IPC,
schema, project format, or design token changed. See
`IMPLEMENTATION_PLAN_v0.1.1.md` and `docs/DECISIONS.md` D-28.

**2026-09-08 · v0.1.0 AI-workflow reliability** — stream listeners are registered
before the backend call and an early first event is retained; the tool-capability
probe allows a short reasoning prelude; Live Illustrator regeneration/questions
and text conversion wait for their listener and show localized failures; the live
acceptance test (`src-tauri/tests/live_ornith.rs`) is environment-configurable and
credential-safe. See `IMPLEMENTATION_PLAN_v0.1.0.md`.

**2026-09-07 · v0.0.6 UI legibility refinement** — standard controls now use a
40 px target, body text uses a 16 px baseline, compact labels follow a consistent
12/13/14 px scale, and important boundaries are reinforced in both themes. SVG
layout and icon stroke weight are normalized, while empty-state content aligns to
the main column. This is a presentation-only follow-up: backend, IPC, routing,
database and project formats are unchanged. See `UI_REFINEMENT_PLAN.md`.

**2026-09-07 · v0.0.5 UI refresh** — the React GUI now uses a neutral,
conversation-first visual system while preserving the existing navigation,
Viewer/Studio hierarchy and accessibility contracts. The change covers shell, tokens,
shared controls, tabs, project cards, Settings, Search, File Modifier, Viewer source rows
and the Studio composer/user-message treatment. `design.md` and `src/styles/tokens.css`
are the current visual sources of truth. No backend, IPC or data-model behavior changed.

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
Tauri v2 + Rust backend + a token-driven React frontend. Since v0.0.3
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
src/                     React frontend. features/, components/, ipc/, i18n/, stores/, styles/
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
