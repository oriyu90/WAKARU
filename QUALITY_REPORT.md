# WAKARU — Quality report

Cumulative; newest release first.

## v0.1.0 release verification — 2026-09-08

Scope: reliability fixes for local and LAN AI streams, capability detection, and
localized Live Illustrator/text-conversion failures. The application hierarchy,
IPC contracts, database schema, project format, and existing workflows remain
unchanged.

### What changed

- Registered stream event listeners before invoking the backend and preserved an
  event that arrives before React's request-state reset completes.
- Increased the bounded tool-capability probe allowance so compatible local models
  can emit their first tool-call fragment instead of being misclassified.
- Made Illustrator regeneration, questions, and text conversion wait for their
  stream listener, and localized visible stream failures.
- Added an environment-configurable, credential-safe live acceptance test with
  Japanese and English Illustrator grounding plus Studio artifact creation.

### Automated gates

| Gate | Result |
|---|---|
| Frontend typecheck | pass |
| Frontend lint (eslint + design-rules + hardcoded-strings) | pass |
| Frontend unit tests (vitest incl. axe) | 9 passed |
| Contrast | pass; all checked light/dark pairs meet their targets |
| i18n parity | 351 keys × 3 languages |
| Production web build | pass; existing large-chunk advisory only |
| ts-rs binding export | 79 passed; drift 0 |
| Rust `cargo fmt --check` / `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| Rust library tests | 163 passed; 1 network/npx test intentionally ignored |
| Rust phase integration tests | 39 passed |
| Live OpenAI-compatible acceptance test | pass with the owner-authorized local model: discovery, streaming, vision, tools, JSON Schema, Japanese/English Illustrator, injection safety, Studio write, artifact registration, and secret non-disclosure |
| `cargo deny check licenses bans sources` | pass |

### Design and failure review

- Confirmed the existing content-first visual system remains intact in Japanese
  and English, including control boundaries, icon alignment, readable labels,
  keyboard focus, light/dark contrast, and localized connection failures.
- Verified a missing local endpoint reports a localized, user-actionable error
  instead of a raw IPC message.
- The supplied endpoint's embedding route returned a handled request error;
  WAKARU safely retains text-index search rather than treating that optional
  capability as a crash or false success.

### Bundle

| Item | Value |
|---|---|
| App | arm64 `WAKARU.app`, version `0.1.0`; ad-hoc signed; `codesign --verify --deep --strict` passes for the build output and the app inside the mounted DMG |
| `Info.plist` | `CFBundleShortVersionString` 0.1.0; `LSMinimumSystemVersion` 12.0 |
| DMG | `WAKARU_0.1.0_aarch64.dmg`, 23,078,673 bytes; `hdiutil verify` VALID |
| SHA-256 | `ff0fe62ea067f0856d13289ca90b13b3a049123974e6dca52d7b8cad1819b1cb` (basename in `WAKARU_0.1.0_aarch64.dmg.sha256`) |
| Startup probe | mounted-DMG app stayed healthy for 8 seconds and reached `WAKARU backend ready version="0.1.0"`; startup log free of secret patterns |
| Platform | macOS 12+, Apple Silicon; Windows/Linux not built or verified |

No database migration is required. Existing v0.0.0–v0.0.6 projects and settings
open unchanged. The bundle is not Developer ID signed or notarized.

---

## v0.0.6 release verification — 2026-09-07

Scope: UI-only legibility and alignment refinement. The navigation, feature set,
IPC contracts, database schema and project format are unchanged.

### What changed

- Raised standard controls to a 40 px target and body copy to a 16 px baseline.
- Organized supporting labels on a consistent 12/13/14 px scale.
- Strengthened control outlines, input boundaries and separators in both themes.
- Normalized SVG layout, 18 px icon sizing, 1.75 stroke weight and flex behavior.
- Aligned empty-state content with the main page column.

### Automated gates

| Gate | Result |
|---|---|
| Frontend typecheck | pass |
| Frontend lint (eslint + design-rules + hardcoded-strings) | pass |
| Frontend unit tests (vitest incl. axe) | 9 passed |
| Contrast | pass; all checked light/dark pairs meet their targets |
| i18n parity | 351 keys × 3 languages |
| Production web build | pass |
| ts-rs binding export | 79 passed; drift 0 |
| Rust `cargo fmt --check` / `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| Rust tests | 163 library + 39 integration passed; 2 network/live-endpoint tests remain `#[ignore]` |
| `cargo deny check licenses bans sources` | pass |

### Visual verification

- Verified Home, navigation and Settings at narrow width in a real browser.
- Checked light and dark themes, 40 px control geometry, 18 px icon centering,
  stronger boundaries, readable supporting text and content-column alignment.

### Bundle

| Item | Value |
|---|---|
| App | arm64 `WAKARU.app`, version `0.0.6`; ad-hoc signed; `codesign --verify --deep --strict` passes for the build output and the app inside the mounted DMG |
| `Info.plist` | `CFBundleShortVersionString` 0.0.6; `LSMinimumSystemVersion` 12.0 |
| DMG | `WAKARU_0.0.6_aarch64.dmg`, 23,080,551 bytes; `hdiutil verify` VALID |
| SHA-256 | `ff9ac1b1163113aa265091e41343e973594dcd18da58aaab0ba5c4b245ebf996` (basename in `WAKARU_0.0.6_aarch64.dmg.sha256`) |
| Startup probe | process stayed healthy for 6 seconds and reached `WAKARU backend ready version="0.0.6"`; startup log free of secret patterns |
| Platform | macOS 12+, Apple Silicon; Windows/Linux not built or verified |

No database migration is required. Existing v0.0.0–v0.0.5 projects and settings
open unchanged. The bundle is not Developer ID signed or notarized.

---

## v0.0.5 release verification — 2026-09-07

Scope: UI-only refresh across the existing application hierarchy. The navigation,
feature set, IPC contracts, database schema and project format are unchanged.

### What changed

- Reworked the application shell into a quieter, conversation-first layout with
  clearer content hierarchy, compact navigation and consistent page widths.
- Unified buttons, fields, cards, tabs, toolbars, empty states and focus states
  around one neutral token system in light and dark themes.
- Refined Home, Settings, Search, File Modifier, Project, Viewer, Source List and
  Studio without moving or removing their existing capabilities.
- Improved Studio's message flow, composer placement and supporting side panels.
- Updated the implementation, UI specification, decision and handoff documents.

### Automated gates

| Gate | Result |
|---|---|
| Frontend typecheck | pass |
| Frontend lint (eslint + design-rules + hardcoded-strings) | pass |
| Frontend unit tests (vitest incl. axe) | 9 passed |
| Contrast | pass; all checked light/dark pairs meet their targets |
| i18n parity | 351 keys × 3 languages |
| Production web build | pass |
| ts-rs binding export | 79 passed; drift 0 |
| Rust `cargo fmt --check` / `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| Rust tests | 163 library + 39 integration passed; 2 network/live-endpoint tests remain `#[ignore]` |
| `cargo deny check licenses bans sources` | pass |

### Visual verification

- Verified the redesigned Home, navigation shell and Settings surfaces in a real
  browser in both light and dark themes.
- Confirmed responsive hierarchy, keyboard focus visibility, control spacing and
  content-column alignment while preserving the existing screen structure.

### Bundle

| Item | Value |
|---|---|
| App | arm64 `WAKARU.app`, version `0.0.5`; ad-hoc signed; `codesign --verify --deep --strict` passes for the build output and the app inside the mounted DMG |
| `Info.plist` | `CFBundleShortVersionString` 0.0.5; `LSMinimumSystemVersion` 12.0 |
| DMG | `WAKARU_0.0.5_aarch64.dmg`, 23,079,059 bytes; `hdiutil verify` VALID |
| SHA-256 | `66d0ea04f3f10673b87b74d2072a06a1b09ed7399c5d9b2c7540ba803b133c19` (basename in `WAKARU_0.0.5_aarch64.dmg.sha256`) |
| Startup probe | process stayed healthy for 6 seconds and reached `WAKARU backend ready version="0.0.5"`; startup log free of secret patterns |
| Platform | macOS 12+, Apple Silicon; Windows/Linux not built or verified |

No database migration is required. Existing v0.0.0–v0.0.4 projects and settings
open unchanged. The bundle is not Developer ID signed or notarized.

---

## v0.0.4 release verification — 2026-09-03

Scope: OCR for scanned PDFs and images, and a `build_document` Studio tool
(Markdown / DOCX / PDF). Additive only — `project.db` gains one nullable
`sources.ocr_status` column (`PROJECT_SCHEMA_VERSION` unchanged); v0.0.0–v0.0.3
projects and settings open unchanged and a round-trip preserves the column.

### What changed

- **OCR engine** — `services/ocr.rs`: `ocrs` + `rten` (pure Rust, no ONNX/C
  deps). Detection + recognition models download once into
  `<data_dir>/models/ocr/` (the local-embedding pattern), behind a process-wide
  lock, with a 30 MP input cap. Missing model / offline / decode failure all
  degrade — never a panic. `OcrPage::looks_like_text()` (min length, ≥ 55 %
  alphanumeric, ≥ 3 distinct chars) rejects OCR hallucination from noise.
- **Image OCR** (`ingest/image.rs`) — an imported image with text gets a
  searchable `ocr` document unit plus `derived/<sid>/ocr.txt` and `ocr.json`
  sidecars. Gated by the new `ocr.enabled` setting (default on).
- **Scanned-PDF OCR** — `ingest/pdf.rs` marks a PDF with any text-less page as
  `ocr_status = "pending"`. `commands/ocr.rs` (`ocr_page` / `ocr_finalize`) +
  `services/ocr.rs::apply_pdf_page` OCR a page raster from the Viewer, and
  **only when the page carries the ingest "no text layer" placeholder** replace
  `documents.text`, rebuild that document's chunks and its `global_index` row
  (so scanned pages become searchable — closes D-09). `finalize_pdf` sets
  `ocr_status` and builds `derived/<sid>/searchable.pdf` (page raster + invisible
  OCR text layer via `pdf_text::render_sandwich`), then drops the page rasters.
- **Viewer** — `PdfFilePreview` shows a "Recognise text" button when
  `ocrStatus` is `pending`/`partial`; it rasterises each text-less page with the
  existing PDF.js pipeline (skipping pages that already have > 100 chars of text
  layer), feeds `ocr_page`, shows N/M progress, then `ocr_finalize`.
- **`build_document`** — `services/doc_builder.rs`: deterministic assembly from
  `{ title, toc, sections:[{level, heading, body}] }` where `body` is a small
  Markdown subset (paragraphs, `-`/`1.` lists, `| tables |`, `**bold**`,
  `*italic*`, `` `code` ``; unhandled syntax is escaped through). Markdown +
  DOCX (`docx-rs`) + PDF (`pdf_text`, `printpdf` with a CJK-capable **system**
  font found via `fontdb` — no font bundled; falls back to `.md` when none is
  found). Wired into Studio's `tool_defs` / `dispatch_tool`, sharing
  `write_artifact` and the sandbox path guard + approval with `write_file`. A
  compact "document skill" was added to `prompts/studio.{en,ja,zh-Hans}.md`.
- **New deps** — `ocrs` / `rten` (MIT), `docx-rs` (MIT), `fontdb` (MPL-2.0,
  in the `deny.toml` allowlist), `ttf-parser` (MIT). `cargo deny` green.

### Automated gates

| Gate | Result |
|---|---|
| Frontend typecheck | pass |
| Frontend lint (eslint + design-rules + hardcoded-strings) | pass |
| Frontend unit tests (vitest incl. axe) | 9 passed |
| Contrast | pass |
| i18n parity | 351 keys × 3 languages |
| Production web build | pass |
| ts-rs binding export | drift 0 |
| Rust `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` | pass |
| Rust tests | 163 lib + 38 integration passed (incl. new `services::ocr`, `services::doc_builder`, `services::pdf_text`, `storage` 003_ocr round-trip, `phase6::build_document`); official-network + live-AI tests remain `#[ignore]` |
| `cargo deny check licenses bans sources` | pass |

### Functional verification (local)

- `services::ocr` ran end-to-end in the integration environment — models
  downloaded and inference executed; the `looks_like_text` guard rejects a
  text-free gradient image.
- `build_document` produced a valid `.docx` (zip with `word/document.xml`) and,
  on this machine, a valid multi-page `%PDF-` with an embedded font subset and
  correct pagination for mixed JP + EN + lists + code.
- `003_ocr` migration: fresh DB gets the column + ledger row; a DB that already
  has the column adopts `003_ocr` without re-`ALTER`ing.

### Bundle

| Item | Value |
|---|---|
| App | arm64 `WAKARU.app`, version `0.0.4`; ad-hoc signed; `codesign --verify --deep --strict` passes for the build output and the app inside the mounted DMG; `spctl` rejects (expected — not notarized) |
| `Info.plist` | `NSLocalNetworkUsageDescription` present; `CFBundleShortVersionString` 0.0.4; `LSMinimumSystemVersion` 12.0 |
| DMG | `WAKARU_0.0.4_aarch64.dmg`, 24,040,500 bytes; `hdiutil verify` VALID |
| SHA-256 | `8e359f20ca3250352cfc4a1eae507596c2f4a83ff8523da926edb9e629e25bd1` (basename in `WAKARU_0.0.4_aarch64.dmg.sha256`) |
| Startup probe | reached `WAKARU backend ready version="0.0.4"`; startup log free of secret patterns |
| Repository | `oriyu90/WAKARU` is **private** for this release cycle — the DMG on the Releases page is collaborator-only |

### Not exercised in this run

- [ ] Live click-through of the Viewer "Recognise text" flow inside the packaged
      app against a real scanned PDF (the `ocr_page` command path and the
      OCR engine are covered by tests; the PDF.js raster loop is not).
- [ ] `build_document` `pdf` on a machine with no CJK system font (the fallback
      to `.md` is covered by `phase6`).

No reproducible crash, data-loss defect, high-severity security defect or open
automated regression remains.

---


## v0.0.3 release verification — 2026-09-03

Scope: local-network AI connection fix, responsive/centred layout, dark-theme
contrast, and a macOS-native visual register — on the existing Tauri v2 + Rust +
React codebase. No database, migration or IPC-contract change; v0.0.0–v0.0.2
projects and settings open unchanged. From this release the application source is
public (MIT); see `docs/DECISIONS.md` D-21.

### What changed

- **LAN AI connections** — the `AiClient` reqwest client is built with
  `.no_proxy()`, so a user-configured endpoint on the LAN or loopback is reached
  directly instead of through a system/`*_PROXY` proxy that cannot route to a
  private address (D-18). `src-tauri/Info.plist` now declares
  `NSLocalNetworkUsageDescription` (JA + EN), so modern macOS can prompt for and
  grant Local Network access (D-19). Confirmed present in the built and
  DMG-mounted `Contents/Info.plist`.
- **Connection diagnostics** — `probe::probe` classifies a transport failure
  (refused / timeout / DNS / TLS), returns a secret-free actionable note, and
  no longer runs the retry back-off on a dead endpoint. `AiSettings` shows a
  localized hint (ja/en/zh-Hans). +2 Rust regression tests.
- **Responsive layout** — Home, Settings, Search and File Modifier centre within
  a window-fluid `--page-max`; Studio uses a centred `--conversation-max` reading
  column with sunk, visibly separated side rails; layout breakpoints stay in
  `rem` so they track the display scale.
- **Dark contrast** — dark `--color-rule` 32→40 %, `--color-rule-strong` 55→61 %,
  `--color-muted` 73→80 %, `--color-neutral` 62→68 %, wider paper steps, body
  weight 350→400. `design.md` re-synced with `tokens.css` (exact-white ink).
  `check-contrast.mjs` extended with 6 stricter pairs.
- **macOS-native register (D-20)** — `-apple-system` (San Francisco) primary UI
  face with the bundled Geist fallback intact (I-2); overlay title bar
  (`titleBarStyle: "Overlay"`) with a draggable unified toolbar and a
  traffic-light inset applied only inside the shell; control height 2 rem, radii
  6/10/12, 3 px accent focus ring; stronger sidebar vibrancy; NSSegmentedControl-
  style segmented control. Hallmark identity (warm paper, single ≤3 % accent, 8
  states, monochrome, i18n, WCAG AA, rem) unchanged.

### Automated gates

| Gate | Result |
|---|---|
| Frontend typecheck | pass |
| Frontend lint (eslint + design-rules + hardcoded-strings) | pass |
| Frontend unit tests (vitest incl. axe) | 9 passed, 0 failed |
| Contrast (`check-contrast.mjs`) | pass; dark primary text exact white; all 34 checked pairs meet WCAG targets |
| i18n parity | 341 keys × 3 languages (ja / en / zh-Hans) |
| Production web build | pass |
| ts-rs binding export | no contract change; drift 0 |
| Rust `cargo fmt --check` | pass |
| Rust `cargo clippy --all-targets -- -D warnings` | pass |
| Rust tests | 149 lib + 38 integration passed, 0 failed (official-network MCP and owner-authorized live-AI tests remain `#[ignore]`) |
| `cargo deny check licenses bans sources` | pass |

### Responsive / theme checks

Verified in a real browser against the dev build at 1440×900, 1280×820 and
390×844, at light and dark, plus the v0.0.1/v0.0.2 matrices below. Home,
Settings and AI settings centre their content with no right-edge void; the
narrow layout collapses the settings rail to a horizontal scroller with no
content clipping. Studio's centred-column layout and the overlay title bar are
exercised only in the packaged app (no Tauri APIs in a plain browser).

### Bundle

| Item | Value |
|---|---|
| App | `src-tauri/target/release/bundle/macos/WAKARU.app`, optimized arm64, version `0.0.3` |
| Signature | ad-hoc (`identity "-"`); `codesign --verify --deep --strict` passes for the build output and for the app inside the mounted DMG; `spctl` rejects (expected — not notarized) |
| `Info.plist` | `NSLocalNetworkUsageDescription` present; `CFBundleShortVersionString` 0.0.3; `LSMinimumSystemVersion` 12.0 |
| DMG | `WAKARU_0.0.3_aarch64.dmg`, 22,340,233 bytes; `hdiutil verify` VALID; built from the signed `.app` + Applications symlink via `hdiutil` |
| SHA-256 | `06a8c55753f0867ebb46d82c4fa9ce8ce5f6c34495ca0fa48851c36cdb0a0408` (recorded as basename in `WAKARU_0.0.3_aarch64.dmg.sha256`) |
| Startup probe | native app reached `WAKARU backend ready version="0.0.3"` and stayed healthy > 7 s; startup log free of secret patterns |
| Platform | macOS 12+, Apple Silicon; Windows/Linux not built or verified |

### Owner-side verification (pre-release)

- The owner ran `npm run tauri build` and confirmed the LAN AI connection
  (`http://192.168.0.165:1234/v1` + key) succeeds from the packaged app after
  granting macOS Local Network access — the defect this release targets.

### Static UI verification (replaces the interactive GUI pass)

The v0.0.3 UI changes were verified by source inspection and static tooling
rather than a manual click-through:

- **Toolbar drag region** — `AppShell.module.css` scopes `-webkit-app-region: drag`
  to `.topBar` and `no-drag` to `.topBar :global(button)`, `:global(a)` and
  `.kbd`. Every interactive child of `.topBar` in `AppShell.tsx` is covered:
  the menu `IconButton` (renders a `<button>`), the `⌘B` `<kbd className={kbd}>`,
  and the settings `<NavLink>` (renders an `<a>`). The wordmark/tagline/spacer
  spans are non-interactive and remain draggable.
- **Traffic-light inset** — `main.tsx` sets `document.documentElement.dataset.tauri
  = "true"` only when `inTauri`; `base.css` raises `--titlebar-inset-start` to
  4.75rem under `:root[data-tauri="true"]`; `tokens.css` keeps the browser default
  at `--space-xs`; `.topBar` consumes it via `padding-inline-start`. So the inset
  applies in the packaged app and not in a plain browser.
- **Overlay title bar** — `tauri.conf.json` `titleBarStyle: "Overlay"` +
  `hiddenTitle: true`; the release build parsed the schema and produced a working
  app (startup probe passed).
- **Studio centred column** — `.messages` is `flex-direction: column; align-items:
  center` and `.messages > *` gets `width: 100%; max-inline-size:
  var(--conversation-max)` (48rem). Its DOM children are the `MessageRow`
  `<article>`, the `.streaming` block and the `.thinking` line — all bounded and
  centred. `.wsBanner` and `.composer` (direct children of `.conversation`, a
  flex column) each carry `width: 100%; max-inline-size: var(--conversation-max);
  margin-inline: auto`. Rails/workspace sit at `--color-paper-2` with
  `--color-rule-strong` separators against the `--color-paper` conversation.
- **Page centring** — Home / Settings panel / Search / File Modifier carry
  `margin-inline: auto` within a bounded measure; verified live at 1440 / 1280 /
  390 in light and dark.
- **Gates** — `check-design-rules` (no `position: fixed`, px only in allowed
  contexts), `check-contrast` (34 pairs), `check-hardcoded`, `check-i18n`
  (341 × 3), eslint and typecheck all pass on the changed files.
- **Hit targets** — `--control-h` 2rem with the `.iconBtn::before` negative-inset
  expansion still yields the 2.75rem `--hit-min` floor.

### `docs/09 §8` 11-step smoke — static pass (replaces the interactive run)

Each step mapped to its code path + a passing integration test, and checked for
v0.0.3 interaction. v0.0.3's only functional change is `AiClient::new` gaining
`.no_proxy()` (unconditional, non-failing); everything else is CSS / a bundle
plist / a boot-time attribute.

| Step | Static evidence |
|---|---|
| 1 · cold start | startup probe: `WAKARU backend ready version="0.0.3"`, healthy > 7 s, no crash |
| 2 · create project | `phase1::create_project`, `ac_1_1_create_makes_a_self_contained_folder` |
| 3 · import PDF/image/mp4/xlsx at once | `phase1::add_and_ingest`, `ac_1_3`, `image_ingest_normalises…`, `phase5::decodes_a_wav…`; `services/ingest/*` unchanged |
| 4 · page-flip / search during analysis (no freeze) | ingest runs on `tokio::spawn` jobs; Viewer + Search are independent reads; `ac_1_11…work_offline`; no shared lock; CSS-only v0.0.3 change |
| 5 · open PDF, Live Illustrator ×3 + ask | `phase4::ac_4_5…persist`, `fr_l6_import_to_studio`; `AiClient` `.no_proxy()` is non-failing and the owner confirmed live AI from the packaged app; drawer gained `aria-live="polite"` only |
| 6 · restart → tabs + conversations restored | `phase2::ac_2_6_tabs_persist_across_reopen`, `phase4::ac_4_5…`, `phase6::ac_6_1…`; `project.db` persistence unchanged |
| 7 · Studio summary → add to source | `phase6::ac_6_10…keeps_its_workspace_files_and_artifacts`; `studio.rs` `import_artifact_as_source` unchanged |
| 8 · monochrome ON + 150 % + Chinese | `:root[data-monochrome="true"] #app{filter:grayscale(1)}` intact and cascades to `<video>`/`<img>` (no `isolation`, no competing `filter`); `[data-scale="150"]` intact; all dims are `rem`/`clamp`; v0.0.3 breakpoints kept in `rem` so panels auto-collapse before the 960-wide min window overflows at 150 %; `check-i18n` 341 × 3 + `check-hardcoded` pass |
| 9 · network off — view / search / export | `ac_1_11_ingest_and_search_work_offline`; local `embed_local` fallback; export is a local zip; unchanged |
| 10 · export → delete → import → intact | `phase10::ac_10_1_export_import_restores…`, `ac_10_3_a_future_major_schema_is_rejected`, `phase1::ac_1_10_delete_removes…` |
| 11 · logs — no secrets | `logging.rs` + `sanitise()`; startup log scanned (0 patterns); `probe::probe`'s `transport_hint` uses only connection-level error text, asserted secret-free by `probe_reports_the_transport_reason_without_retrying` |

Security AC-7-5 (all 10 path-traversal cases) + AC-7-6 symlink: `sandbox::
resolve_in_sandbox` unit tests + `phase6::write_file_rejects_paths_outside_the_workspace`
+ `phase7` — all green, unchanged in v0.0.3.

### §6 / §7 checklist spot-checks (static)

- No `position: fixed` in `src/` (comment only); `check-design-rules` enforces it.
- No raw `#000` / `#fff` / black-or-white `rgb()` in any component CSS.
- No italic headings (`base.css` sets `h1–h6 { font-style: normal }`; grep confirms 0).
- `[data-scale]` (6 steps) and `[data-monochrome] #app { filter: grayscale(1) }` intact.
- `prefers-reduced-motion: reduce` blocks intact (`base.css` + `AppShell.module.css`).
- Streaming regions are `aria-live="polite"`: Studio `.messages` (existing) and the
  Live Illustrator drawer body (**added in v0.0.3** — was missing).
- Icon-only buttons: `IconButton` requires a `label` prop and always emits
  `aria-label`; loading buttons use `aria-busy`.
- Contrast recomputed from `tokens.css`: 34 pairs, all ≥ target (dark ink exact white).

### Still deferred

- Live Illustrator against a non-mock remote model (prompt policy is unit tested
  in all three response languages; the owner's live run against the LAN model
  covered the transport and grounding paths).
- Determinate `role="progressbar"` on long-running progress (whisper model
  download, ingest): currently indeterminate `aria-busy` / `Spinner`. Non-blocking
  a11y polish, not a v0.0.3 regression.

No reproducible crash, data-loss defect, high-severity security defect or open
automated regression remains.

---

# WAKARU v0.0.2 — Quality report (historical)

Generated 2026-09-02 from automated gates, responsive browser checks, native
macOS startup, log review, dependency audit and final DMG verification.

## v0.0.2 release verification — 2026-09-02

- Version metadata is aligned at `0.0.2` in npm, Cargo and Tauri manifests.
- The Viewer now dispatches original PDF, DOCX, PPTX, XLSX/XLS, CSV/TSV,
  Markdown, image and media sources to dedicated renderers. Original files stay
  behind the project-scoped asset protocol; rich-preview failures fall back to
  extracted text.
- Former deferrals are resolved by local fastembed search, structured image
  Vision augmentation, Studio text/tool events, MCP Streamable HTTP, Silero VAD
  and bounded runtime video thumbnails. D-17 records the exact scope.
- A strict review found and fixed one design warning: the Studio approval-resume
  path exceeded the maintained function-argument limit after streaming support
  was added. Its inputs are now a typed request object; clippy passes with every
  warning denied.

### v0.0.2 automated gates

| Gate | Result |
|---|---|
| Frontend typecheck, lint and design rules | pass |
| Frontend unit tests | 9 passed, 0 failed |
| Contrast | pass; dark primary text remains exact white, all checked pairs meet WCAG targets |
| i18n parity | 339 keys × 3 languages |
| Production web build | pass; document renderers and bundled PDF worker emitted as production assets |
| npm audit (all / production) | 0 vulnerabilities |
| Rust format and clippy (`-D warnings`, all targets/features) | pass |
| Rust tests | 185 passed, 0 failed; official-network MCP and owner-authorized live AI tests remain ignored by default |
| cargo-deny licenses, bans and sources | pass |
| ts-rs binding export | pass; canonical IPC binding updated |

Responsive shell checks passed at 320×900, 600×1200 and 2560×600 with document
width equal to viewport width and no right/bottom truncation. These supplement
the v0.0.1 matrix below. The native v0.0.2 development app and the final app
inside the DMG both reached backend-ready state and remained healthy through
the startup probe. macOS screen-capture/accessibility permission was unavailable,
so rich file rendering was verified through production build paths and renderer
state/error review rather than automated clicks inside the native WebView.

### v0.0.2 bundle

| Gate | Result |
|---|---|
| Release build | pass; optimized arm64 `WAKARU.app`, version `0.0.2` |
| Signature | ad-hoc; `codesign --verify --deep --strict` passes for build output and mounted-DMG app |
| DMG | `WAKARU_0.0.2_aarch64.dmg`, 21,391,026 bytes; `hdiutil verify` VALID |
| Mounted-DMG startup | healthy for 6 seconds; clean termination; backend reported version `0.0.2` |
| Startup secret scan | no API key, authorization header, bearer token or supplied LAN key pattern |
| SHA-256 | `b46ef031b81578db4bdcccf01da7210f5b9091789c74c92a1704be6a2614a225` |

The size increase from v0.0.1 is expected: v0.0.2 bundles the local embedding
runtime and document-rendering assets. The embedding model itself is downloaded
to application data only when first needed and is not inside the DMG.

The first post-publication download check found that the checksum manifest named
the DMG with its build-directory path. The digest itself matched, but a user
could not run `shasum -c` directly beside the downloaded files. The manifest was
replaced with the basename-only form and downloaded again; direct verification
then returned `WAKARU_0.0.2_aarch64.dmg: OK`. The DMG was unchanged.

Known limits are explicit rather than treated as open code defects: no Developer
ID/notarization, no Windows/Linux release verification, no automatic OCR index
for scanned PDF pages without a text layer, and no promise to decode encrypted,
DRM or browser-unsupported legacy formats.

## Previous v0.0.1 verification

## v0.0.1 release verification — 2026-09-02

- Version metadata is aligned at `0.0.1` in npm, Cargo, and Tauri manifests.
- v0.0.1 includes the responsive/LAN API/MCP re-audit below and all fixes from
  v0.0.0. Existing application data formats remain compatible.
- The final automated gates, native bundle checks, DMG checksum, and published
  release-asset verification are recorded in the release section below.

## 2026-09-02 responsive / LAN API / MCP re-audit

- Responsive browser matrix passed at 240×320, 320×568, 568×320, 960×640,
  1280×840 and 1600×500. Home, Settings, File Modifier, Search and the
  component-state preview kept document width equal to viewport width; tall
  content remained reachable through the intended inner/main scrollers.
- 960×640 and 320×568 also passed at 150% display scale. A remaining 4 px
  Search select overflow found only at 320×568/150% was fixed and retested.
- OpenAI-compatible and Anthropic-compatible profile paths were statically
  re-audited. LAN `http://` URLs remain valid, `/v1` remains an explicit part
  of the configured base URL, headers and timeout bounds are validated, and
  auth/protocol headers cannot be silently overridden by custom headers.
- Capability probes now require an actual named tool call and parse valid
  schema-constrained JSON; a merely successful HTTP response no longer creates
  those two false-positive capability flags.
- MCP now uses exactly pinned `rmcp 3.0.1`, prefers the MCP 2026-07-28 discover
  lifecycle, and falls back to 2025-11-25 initialization. The official
  `@modelcontextprotocol/server-everything` stdio server passed real tool-list
  discovery and an `echo` call. Timeouts, graceful cleanup, catalog refresh,
  duplicate-name isolation, strict JSON arguments, secret-redacted stderr and
  all standard result content kinds are covered by the implementation.
- Fresh gates: frontend production build, lint/design rules, 9 unit tests,
  contrast and 327×3 i18n parity all pass. Rust fmt and warning-as-error clippy
  pass; 185 normal backend tests pass with 2 explicit live/manual tests ignored,
  and the ignored MCP interoperability test passes when run manually. Cargo
  license, ban and source checks pass.

## Automated gates — all green

### Frontend

| Gate | Command | Result |
|---|---|---|
| Type check | `npm run typecheck` | pass |
| Lint + design rules + no hardcoded strings | `npm run lint` | pass |
| Unit tests | `npm test` (vitest) | 9 passed |
| Contrast (OKLCH → sRGB WCAG recompute) | `npm run check:contrast` | pass — body ≥ 4.5:1, UI edges ≥ 3:1 |
| i18n key parity (en / ja / zh-Hans) | `npm run check:i18n` | pass — 327 keys × 3 |

### Backend (`src-tauri/`)

| Gate | Command | Result |
|---|---|---|
| Rust formatting | `cargo fmt --check` | pass |
| Clippy (all targets, warnings = errors) | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test --all-targets --all-features` | 185 passed, 0 failed; 2 authorised live/manual tests ignored by default |
| Licenses | `cargo deny check licenses` | ok — no GPL/AGPL/LGPL (`deny.toml`) |
| Dependency bans / sources | `cargo deny check bans sources` | ok |
| ts-rs binding drift | `cargo test export_bindings` + git diff | no diff |

Test breakdown: 147 library unit tests + 38 integration tests
(`tests/phase{1..10}.rs`) + 9 frontend unit tests.

New compatibility and reliability coverage includes Anthropic profile migration,
Base URL boundary validation, OpenAI-to-Anthropic system/tool/tool-result
conversion, a real local HTTP/SSE exchange that verifies Anthropic headers,
text, usage and streamed tool arguments, and prompt-policy presence in all
three UI languages. Legacy database fixtures prove that the app adopts the old
schema, preserves rows, and repairs the old FTS shape even when an earlier build
already marked that migration as applied.

The current replacement adds regression coverage for source/system separation
in Illustrator and the organiser, Studio's untrusted project-context boundary,
mandatory complete-file tool guidance for smaller models, suspiciously short
organiser output, exact-white primary dark-theme text, and OpenAI-compatible
`finish_reason: "length"` handling.

An explicitly authorised live test against `Ornith-1.5-35B-A3B-MLX-4bit`
passed model discovery, streaming, Vision and tool probes, grounded
Illustrator output, prompt-injection resistance, and a four-round Studio
workflow that created and registered `kestrel-summary.md`. The endpoint does
not implement `/v1/embeddings`, so search correctly remains FTS-only. See
the private maintainer validation record for the endpoint-specific details.

Security-relevant coverage:

- Path traversal — every `docs/09 §7 (AC-7-5)` case is a unit test in
  `services::sandbox` (absolute, Windows-style, `%`-encoded, `..`, symlink
  escape).
- `run_command` — shell-free (`; rm -rf /` stays a literal arg), environment
  scrubbed to `PATH/HOME/TMPDIR/LANG`, process-group SIGKILL on timeout, 1 MB
  output cap (AC-7-7..7-10).
- Tool-result isolation — MCP / tool output is stored and sent as
  `role: "tool"`; the system prompt states it is data, not instructions
  (AC-7-12).
- Citations — `[S9]`-style undefined tags are dropped on the Rust side
  (AC-4-8 / I-5), unit-tested.
- Asset access — `wakaru-asset://` resolves only inside a project's
  `sources/` and `derived/`; traversal rejected (unit-tested).
- Dependency audit — vulnerable `lopdf` and `quick-xml` generations were
  replaced. `cargo deny check` passes; no vulnerability advisory is ignored.
  Maintenance-only advisories without an upstream replacement remain recorded
  with explicit reasons in `src-tauri/deny.toml`.

## Native runtime debugging

The signed production `.app` was operated through the real macOS UI at
1280×840. Fixtures covered Markdown, CSV, JSON, PDF and PNG. A local HTTP
server exercised the OpenAI-compatible models, embeddings and streaming-chat
routes without sending fixture content off-device.

| Scenario | Result |
|---|---|
| Existing v0.0.0 database opens and migrates in place | pass; project/source rows preserved |
| Add and analyse Markdown / CSV / JSON / PDF / PNG | pass; 5 sources, 10 documents, 9 chunks, 15 FTS rows |
| Markdown and CSV production previews | pass; text/table rendered after custom-protocol CORS repair |
| PDF and image previews | pass; extracted PDF text and PNG pixels rendered |
| Restart and tab/project persistence | pass |
| OpenAI-compatible profile add, connection test and four role assignments | pass; model and Vision/Tools/Embedding capabilities detected |
| Studio request over the configured endpoint | pass; source-grounded answer, `[S1]` citation and Socratic check displayed |
| FTS-only operation without an embedding profile/table | pass; clean no-op, no missing-table warning |
| Search-bar usability at desktop and narrow widths | pass after flex sizing repair |
| Current replacement starts from the signed production bundle | pass; native window opened and exited normally |
| Dark-theme primary text and File Modifier layout | pass in local production-equivalent web UI; primary tokens resolve to exact white |

### Defects found and fixed before replacing v0.0.0

1. **Critical — all imports could fail on an adopted legacy database.** The old
   `global_index` FTS table lacked `body_raw`, while the ledger incorrectly made
   the schema look current. Startup now detects and transactionally rebuilds
   that table, preserving indexed rows. Never-adopted and already-adopted legacy
   shapes both have regression tests.
2. **High — Markdown/CSV previews failed only in the signed app.** WKWebView
   rejected `fetch(wakaru-asset://...)` because custom-protocol responses lacked
   CORS headers. GET/error responses now include the allow-origin header and
   OPTIONS is handled explicitly.
3. **Medium — keyword-only ingestion logged a false database warning.** SQLite
   resolved a reference to missing `chunk_vectors` while preparing an `OR`
   expression. The query is now selected only after checking table existence.
4. **High — global search input collapsed to roughly 26 px.** Competing 100%
   widths on input/select controls caused flex shrinkage. The search bar now has
   explicit flex bases, minimum widths and a narrow-screen stacked layout.
5. **High/security — crafted PDF/XML could terminate or stall parsing.** PDF,
   Office, XML and HTML dependencies were upgraded to fixed generations;
   image-to-PDF was ported to the current API and retested.

## Build & bundle — v0.0.1 arm64

| Gate | Result |
|---|---|
| `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app` | pass |
| Bundle | `WAKARU_0.0.1_aarch64.dmg`, `WAKARU.app`; arm64, version `0.0.1`, ad-hoc signed, **not notarized** |
| `hdiutil verify` | checksum VALID |
| `codesign --verify --deep --strict` — build output and mounted-DMG app | valid on disk, satisfies its Designated Requirement |
| Mounted-DMG launch | process remained healthy for the 6-second startup probe; terminated cleanly by the test |
| Startup-log secret scan | 0 API-key/auth-header patterns |
| Signature | `adhoc`, Identifier `com.yukiorita.wakaru`, TeamIdentifier not set |
| Size | 11,525,041 bytes |
| `shasum -a 256` | `1b5f0d7200880f008cfbc4e9fde6b8c83629dd506ae7b8ec320cdcfcae32fe84` |

The official `@modelcontextprotocol/server-everything` ignored interoperability
test was run explicitly for the release build and passed discovery, tool-list
retrieval, and an `echo` call.

## Previous build record — v0.0.0 arm64

| Gate | Result |
|---|---|
| `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app` | pass |
| Bundle | `WAKARU_0.0.0_aarch64.dmg`, `WAKARU.app`; ad-hoc signed, **not notarized** |
| `hdiutil verify` | checksum VALID |
| `codesign --verify --deep --strict` — `.app` in `bundle/macos/` | valid on disk, satisfies its Designated Requirement |
| `codesign --verify --deep --strict` — `.app` **inside the mounted DMG** | valid on disk, satisfies its Designated Requirement |
| `spctl -a -t exec` | **rejected** — expected for ad-hoc/unnotarized; README documents right-click → Open |
| Signature | `adhoc`, Identifier `com.yukiorita.wakaru`, TeamIdentifier not set |
| Size | 11,419,349 bytes |
| `shasum -a 256` | `d63a72a9b8f11b88e763bf4cbc763c2a695d0b0ae456cd5c01db73a9591cbc34` |

x64 build not attempted (only arm64 is distributed, matching v0.1.0).

Build note: `whisper.cpp` (ggml) needs a macOS 10.15+ deployment target for
`<filesystem>`; `tauri build` otherwise passes 10.13. Fixed permanently via
`bundle.macOS.minimumSystemVersion = "12.0"` + a forced
`MACOSX_DEPLOYMENT_TARGET` in `src-tauri/.cargo/config.toml`.

The Tauri DMG decoration helper failed while automating Finder layout. The
release DMG was therefore created from the same signed `.app` with the standard
Applications symlink using `hdiutil`; image integrity and the mounted app's
signature were then independently verified.

## Not exercised in this release run

- [ ] Full export/delete/import round-trip through native file dialogs (the
      backend integration round-trip passes).
- [ ] Live Illustrator against a non-mock remote model (prompt policy is unit
      tested in all three response languages).
- [ ] `docs/09 §6` Hallmark checklist on every screen.
- [ ] `docs/09 §7` accessibility checklist (focus rings, arrow-key tab groups,
      focus trap + Esc, `aria-label` on icon buttons, `role="progressbar"`,
      `aria-live="polite"`, `prefers-reduced-motion`, 2rem hit targets).
- [ ] Cold start < 3 s on an empty library.

These broader fixture/API-dependent checks remain release-candidate follow-up
work. No reproducible crash, data-loss defect, high-severity security defect,
or automated regression remains open; v0.0.1 is published as a stable release.
