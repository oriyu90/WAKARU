# WAKARU v0.0.0 — Quality report

Generated 2026-09-01 from a clean run of every automated gate, native macOS
interaction tests against a fixture library and local compatible API, database
migration verification, log review, and final DMG verification.

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
| Tests | `cargo test --all-targets --all-features` | 183 passed, 0 failed; 1 authorised live test ignored by default |
| Licenses | `cargo deny check licenses` | ok — no GPL/AGPL/LGPL (`deny.toml`) |
| Dependency bans / sources | `cargo deny check bans sources` | ok |
| ts-rs binding drift | `cargo test export_bindings` + git diff | no diff |

Test breakdown: 145 library unit tests + 38 integration tests
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

## Build & bundle — v0.0.0 arm64

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
or automated regression remains open; v0.0.0 is published as a pre-release.
