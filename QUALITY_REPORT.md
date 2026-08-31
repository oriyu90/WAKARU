# WAKARU v0.0.0 — Quality report

Generated 2026-09-01 from a clean run of every automated gate, browser UI
inspection, native startup/migration verification, and final DMG verification.

## Automated gates — all green

### Frontend

| Gate | Command | Result |
|---|---|---|
| Type check | `npm run typecheck` | pass |
| Lint + design rules + no hardcoded strings | `npm run lint` | pass |
| Unit tests | `npm test` (vitest) | 6 passed |
| Contrast (OKLCH → sRGB WCAG recompute) | `npm run check:contrast` | pass — body ≥ 4.5:1, UI edges ≥ 3:1 |
| i18n key parity (en / ja / zh-Hans) | `npm run check:i18n` | pass — 327 keys × 3 |

### Backend (`src-tauri/`)

| Gate | Command | Result |
|---|---|---|
| Rust formatting | `cargo fmt --check` | pass |
| Clippy (all targets, warnings = errors) | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test --all-targets --all-features` | 175 passed, 0 failed |
| Licenses | `cargo deny check licenses` | ok — no GPL/AGPL/LGPL (`deny.toml`) |
| Dependency bans / sources | `cargo deny check bans sources` | ok |
| ts-rs binding drift | `cargo test export_bindings` + git diff | no diff |

Test breakdown: 137 library unit tests + 38 integration tests
(`tests/phase{1..10}.rs`) + 6 frontend unit tests.

New compatibility and reliability coverage includes Anthropic profile migration,
Base URL boundary validation, OpenAI-to-Anthropic system/tool/tool-result
conversion, a real local HTTP/SSE exchange that verifies Anthropic headers,
text, usage and streamed tool arguments, and prompt-policy presence in all
three UI languages. A legacy untracked database fixture also proves that the
app adopts the old schema, preserves rows, and applies both missing columns
without attempting to recreate existing tables.

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
| Size | 11,230,289 bytes |
| `shasum -a 256` | `92b615bd511f2a226b0c6e5168cd4ba2086062c1d7aa2b4909e0f4255822d672` |

x64 build not attempted (only arm64 is distributed, matching v0.1.0).

Build note: `whisper.cpp` (ggml) needs a macOS 10.15+ deployment target for
`<filesystem>`; `tauri build` otherwise passes 10.13. Fixed permanently via
`bundle.macOS.minimumSystemVersion = "12.0"` + a forced
`MACOSX_DEPLOYMENT_TARGET` in `src-tauri/.cargo/config.toml`.

The Tauri DMG decoration helper failed while automating Finder layout. The
release DMG was therefore created from the same signed `.app` with the standard
Applications symlink using `hdiutil`; image integrity and the mounted app's
signature were then independently verified.

## Interactive verification performed

- Native v0.0.0 app cold-launched and remained running.
- A real legacy application database with an empty migration ledger was backed
  up, upgraded, and reopened successfully. Project/profile row counts stayed
  unchanged; `001_init` and `002_ai_protocol` were recorded; `json_schema` and
  `protocol` were added.
- AI Settings was inspected in the browser build: OpenAI/Anthropic selector,
  Anthropic preset URL, profile badge, save/error affordances, and translations
  were present.
- 390×844 layout and 150% display size had no horizontal overflow; the browser
  console had no errors.

## Not exercised in this release run

- [ ] `docs/09 §8` 11-step smoke test (ingest → view → Illustrator →
      restart-persists → Studio artifact → monochrome / 150% / 中文 → offline
      view+search+export → export/delete/import round-trip → log check).
- [ ] `docs/09 §6` Hallmark checklist on every screen.
- [ ] `docs/09 §7` accessibility checklist (focus rings, arrow-key tab groups,
      focus trap + Esc, `aria-label` on icon buttons, `role="progressbar"`,
      `aria-live="polite"`, `prefers-reduced-motion`, 2rem hit targets).
- [ ] Cold start < 3 s on an empty library.

These broader fixture/API-dependent checks remain release-candidate follow-up
work. No reproducible crash, data-loss defect, high-severity security defect,
or automated regression remains open; v0.0.0 is published as a pre-release.
