# WAKARU v0.0.2 — Quality report

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
