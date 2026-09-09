# WAKARU v0.2.0 — implementation plan

A feature + reliability release. Seven reader-reported problems, an
OpenAI/Anthropic/LM-Studio wire-compat audit, and the dangerous-design items
found along the way. Safe-design contract holds: no project-format change, no
destructive migration, JA / EN / zh-Hans string parity, crash- and
memory-safety, every existing capability kept.

studio-rizi is updated to say **"近日公開 / Coming soon"** for v0.2.0 on the
project card and the intro page (no download link flip until the release is
cut). Actually the DMG *is* built and published to GitHub the same as prior
releases — "coming soon" is only the wording the owner asked for on the site.

---

## A. Reported problems — root cause and fix

### P1 — a opened document in "資料を見る" does not grow with the window
`PdfFilePreview` renders the page at `getViewport({ scale: min(dpr,2) * zoom })`
with `zoom` fixed at `1` — i.e. the PDF is drawn at its native point size and
`.pdfCanvas` has `max-width: none`. On a wide window the page is a small island
of white on black. Same shape for the other renderers to a lesser degree.
**Fix F1**: a `ResizeObserver` on the scroll viewport computes a fit-to-width
scale from the page's intrinsic width; the on-screen scale is
`fitScale * userZoom` (userZoom keeps the − / 100% / + control working as a
multiplier). Clamp so a tiny page doesn't balloon past ~1.75×. DOCX/PPTX/sheet
viewers already flex; only confirm their `max-width`.

### P2 — the Live Illustrator panel and its open control are invisible
The drawer's only affordance is `.handle`, a `0.5rem` bar painted
`--color-paper-2` on a `--color-paper` stage — in dark mode that is ~8 % grey on
black, undiscoverable. And it renders only while `illustratorEnabled`.
**Fix F2**: replace the sliver with a real labelled toggle button in the viewer
tab strip (right-aligned, icon + "ライブ解説"). When Illustrator is **off** the
button turns it on and opens the drawer; when **on** it opens/closes the drawer
(`aria-expanded`). `Cmd/Ctrl+\` keeps working. The drawer keeps its width
transition. This gives one discoverable control that satisfies both halves of
the report.

### P3 — a sent Studio message stays in the composer
`send`'s `onSuccess` clears `text`; on **error** (which P4 makes the common
case) the text is kept, so after a failed turn the message shows both as a
persisted bubble and in the box.
**Fix F3**: clear the composer in `onMutate` (the turn is persisted before the
model call, so it is never lost) and, on error, surface the failure as a
dismissible row in the thread instead of only a toast. A send that fails can be
retried from the thread.

### P4 — LM Studio (and mlx-bar) replies do not come back
Two independent defects in `services/ai/client.rs`:
1. **`truncated` false-positive.** `chat_stream_openai` starts `truncated =
   true` and only clears it on `data: [DONE]`. LM Studio in several configs,
   llama.cpp's server, and other OpenAI-compatible servers close the SSE stream
   cleanly right after a `finish_reason` chunk and never send the `[DONE]`
   sentinel. WAKARU then reports `AI_TRUNCATED`; Studio turns that into an error
   and shows nothing. **Fix**: track a terminal `finish_reason`
   (`stop` / `tool_calls` / `content_filter` / `function_call` / `length`). If
   the stream ends after one of those without `[DONE]`, it is a complete
   response (`length` still means output-limit-reached). `[DONE]` handling is
   unchanged; a genuine mid-stream drop (no finish_reason) is still `truncated`.
2. **Tools rejected by the model.** Studio always sends `tools` +
   `tool_choice:"auto"`. LM Studio returns `400 "this model does not support
   tools"` for GGUF models without a tool template; other servers 400 similarly.
   `supports_tools` on the profile is unreliable (defaults to 0, only set by the
   optional connection test), so gating on it would wrongly disable Studio tools
   for capable-but-untested setups. **Fix**: in `run_loop`, if `chat_stream`
   returns `AI_REQUEST` (400/404/422) while the request carried tools, retry the
   round once with `tools` / `tool_choice` stripped and set a per-run
   `tools_disabled` flag so later rounds skip them too. Studio degrades to plain
   RAG chat instead of failing. A 400 with no tools in the request is surfaced
   unchanged.

Also: `retry()` wraps the chat POST and re-sends on `429/5xx`. Once the server
has accepted the request a 5xx retry means a second full (expensive, local)
generation. **Fix**: the streaming chat POST retries only on connect/timeout
(no bytes yet), not on a 5xx *response*. `list_models` / `embeddings` keep the
5xx retry (idempotent).

### P5 — window chrome: hamburger inset and the traffic lights
`titleBarStyle: "Overlay"` draws the macOS traffic lights inside the webview's
top-left; WAKARU reserves `--titlebar-inset-start: 4.75rem` for them
unconditionally. In **fullscreen** macOS hides the lights, so the reserved gap
just pushes ☰ / the wordmark right for nothing. The lights cannot be moved
outside the window with this style (that is what Overlay means); a fully custom
chrome is out of scope for a maintenance line.
**Fix F5**:
- Listen to the Tauri window resize/fullscreen events; set
  `:root[data-fullscreen="true"]` and, there, `--titlebar-inset-start:
  var(--space-md)` and drop the drag region's reserved space. ☰ sits at the
  normal left edge in fullscreen.
- Windowed: keep the inset (correct — it clears the lights) but tighten it to
  the real light cluster width and vertically centre the top bar contents so the
  lights are not clipped by the `3.5rem` bar. Add `trafficLightPosition` via the
  window builder so the cluster is centred in the taller bar.
- Document in the release notes that the lights are part of the window content
  by design on macOS.

### P6 — some UI does not track the window size
Audit pass. Confirmed offenders beyond P1: none structural, but
`--rail-w` / `--panel-w` (Studio) and the viewer tab strip use fixed rems with
`@media` steps that skip a band around 48–60 rem. **Fix F6**: convert the
Studio rails to `clamp()` widths, give the viewer stage a container query for
the Illustrator drawer breakpoint instead of a viewport `@media`, and re-check
Home / Search / File Modifier at 900 / 1280 / 1600 and 320-wide.

### P7 — "資料を見る" is a ladder of horizontal rules; redesign
Adopt the reader's suggestion. **Fix F7**: a left **vertical tab rail** (like
Studio's) holding: 🏠 home, then one row per open document, then the **追加** and
**リンクを追加** actions pinned at the bottom of the rail. The main area shows
the active document or the source list with far fewer dividers (rule only
between rail and stage, and under the single toolbar). Drag-and-drop, middle-
click-close, citation focus, keyboard order, and the empty state are preserved.
New/changed i18n keys land in all three locales.

---

## B. External wire-format audit (done before coding)

- **LM Studio** (0.3.x, OpenAI-compat `/v1`): sends `data: [DONE]` in current
  builds but not universally; rejects `tools` for models without a tool
  template; accepts `stream_options.include_usage`. → covered by P4.
- **mlx-bar** (`oriyu90/mlx-bar`, `/v1/chat/completions`): strict allowlist —
  `400 UNSUPPORTED_PARAMETER` for any body key outside its set; **does** send
  `[DONE]` and a `finish_reason` chunk; emits `: keep-alive` SSE comments during
  prefill (our `eventsource-stream` ignores comment lines — verified). WAKARU
  only ever sends allowlisted keys (`model, messages, stream, stream_options,
  tools, tool_choice`), so no change needed for the default flow; the P4
  tools-strip fallback also covers a future stricter build.
- **OpenAI**: always sends `[DONE]`. The OpenAI path passes role-binding params
  through verbatim — no `max_tokens` rewrite there (that mapping is Anthropic-
  only), so a binding that sets `max_completion_tokens` reaches an `o*` model
  correctly. Default params are `{}`. **Reviewed — no change needed.**
- **Anthropic** (`/v1/messages`, `2023-06-01`): `anthropic_body` produces
  `model, messages, max_tokens, stream, system` for the default flow — valid.
  `response_format` in a binding is mapped to `output_config.format` (a tested,
  deliberate mapping for the beta structured-output shape); not reachable from
  today's UI. **Reviewed — left as-is** to avoid regressing tested behavior.
- Net: the OpenAI/Anthropic default flows are already correct; the compat gap
  the reader hit is entirely the P4 pair (missing `[DONE]`, tools rejection),
  which is server-agnostic.

---

## C. Dangerous-design / bottleneck items folded in

1. **`studio::list_tabs` loads every message of every tab on each poll**
   (`load_messages` per tab), and the frontend re-queries it after every send,
   tab switch and reload — O(all Studio history) per interaction. **Fix C1**:
   `list_tabs` returns tab metadata + `messageCount` only; a new
   `studio_get_tab(tabId)` returns one tab's messages. `Studio.tsx` loads the
   active tab's messages with its own query. This is an internal IPC-shape
   change (ts-rs regen + frontend update); no schema change, no data migration.
2. **`retry()` around the non-idempotent chat POST** — see P4; restricted to
   connect/timeout.
3. **PDF OCR loop builds a full-page PNG data URL per page on the main thread**
   (`off.toDataURL` at scale 2 for every page) — an 80-page deck spikes hundreds
   of MB and blocks paint. **Fix C3**: cap the raster to ≤ 2000 px on the long
   edge, `await` a `0`-delay yield between pages, and stop early if the tab
   closes. Bounded memory, responsive UI.
4. **`useUiStore.subscribe` writes the DOM on every store change**, including
   `sidebarOpen`. **Fix C4**: subscribe with a selector to the display subset
   only.
5. **`useAssetBuffer` holds up to 200 MB in JS plus a `.slice(0)` copy for
   pdf.js** — documented as a known limit in the plan; raise nothing now, but
   add an explicit user-facing message at the limit (already present) and note
   it. No code change beyond a comment.

---

## D. Verification

Frontend: `typecheck`, `lint`, `test`, `check:contrast`, `check:i18n`, `build`.
Bindings: `npm run bindings`; `git diff` on `types.gen.ts` reviewed (expected to
change — C1 adds `studio_get_tab` and slims `StudioTab`).
Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` (add: `[DONE]`-absent completion, terminal-`finish_reason`
detection, tools-strip retry, OpenAI reasoning-model param gate), `cargo deny`.
Wire: a local mock OpenAI SSE server exercised in an integration test for the
"no `[DONE]`, clean `finish_reason:stop`" and "400 on tools" paths.
Live: browser pass over all seven flows at 900 / 1280 / 1600 wide and 320
narrow, JA and EN, light and dark; PDF fit-to-width; Illustrator toggle;
Studio send/clear/error; fullscreen vs windowed chrome.

## E. Release (v0.2.0)

Version bump in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
`src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`. Signed DMG built and
verified exactly as v0.1.x (`hdiutil verify`, inner-app strict codesign,
startup probe `version="0.2.0"`, `shasum`). `gh release create v0.2.0 --latest`
with the DMG + `.sha256`. Docs: `RELEASE_NOTES.md`, `QUALITY_REPORT.md`,
`docs/DECISIONS.md` (D-31), `docs/HANDOFF.md`, `README.md`, `RELEASE_DRAFT.md`,
`THIRD_PARTY_LICENSES.md` (regen; expect no change).
studio-rizi: version strings to `0.2.0`, **"近日公開 / Coming soon / 即将推出 /
Em breve"** on the wakaru card (`content.js`) and the four intro pages' hero
note, a 4-language NEWS entry; `npm test` + build; push; confirm Cloudflare.
common-rules-document `WAKARU.md`: new dated entry + refreshed status block.
Repo stays private; the private memo is never copied to public surfaces.

## F. Safety notes

Compatibility: data from v0.0.0–v0.1.3 loads unchanged; the only IPC change is
additive-plus-slim on Studio tab payloads (internal). Crash safety: the new
`ResizeObserver`, Tauri event listeners and `ContextMenu`-style portals are all
torn down on unmount; no new `unwrap` on external input; the tools-strip retry
is bounded to one extra attempt. Memory safety: no new `unsafe`; no Rust
signature exposed across FFI changes beyond serde-derived types. i18n: every new
key in ja/en/zh-Hans, enforced by `check-i18n`.
