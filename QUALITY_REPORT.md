# WAKARU v0.2.0 — Quality report

Generated 2026-09-01 from a clean run of every automated gate. Manual gates
(11-step smoke test, Hallmark/A11y checklists, DMG verification) are run on the
release machine and their outcome is appended here before the GitHub release is
published.

## Automated gates — all green

### Frontend

| Gate | Command | Result |
|---|---|---|
| Type check | `npm run typecheck` | pass |
| Lint + design rules + no hardcoded strings | `npm run lint` | pass |
| Unit tests | `npm test` (vitest) | 6 passed |
| Contrast (OKLCH → sRGB WCAG recompute) | `npm run check:contrast` | pass — body ≥ 4.5:1, UI edges ≥ 3:1 |
| i18n key parity (en / ja / zh-Hans) | `npm run check:i18n` | pass — 320 keys × 3 |

### Backend (`src-tauri/`)

| Gate | Command | Result |
|---|---|---|
| Clippy (all targets, warnings = errors) | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test` | 167 passed, 0 failed |
| Licenses | `cargo deny check licenses` | ok — no GPL/AGPL/LGPL (`deny.toml`) |
| Dependency bans / sources | `cargo deny check bans sources` | ok |
| ts-rs binding drift | `cargo test export_bindings` + git diff | no diff |

Test breakdown: 129 library unit tests + 38 integration tests
(`tests/phase{1..10}.rs`) + 6 frontend unit tests.

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

## Manual gates — to complete on the release machine

- [ ] `docs/09 §8` 11-step smoke test (ingest → view → Illustrator →
      restart-persists → Studio artifact → monochrome / 150% / 中文 → offline
      view+search+export → export/delete/import round-trip → log check).
- [ ] `docs/09 §6` Hallmark checklist on every screen.
- [ ] `docs/09 §7` accessibility checklist (focus rings, arrow-key tab groups,
      focus trap + Esc, `aria-label` on icon buttons, `role="progressbar"`,
      `aria-live="polite"`, `prefers-reduced-motion`, 2rem hit targets).
- [ ] Cold start < 3 s on an empty library.
- [ ] `APPLE_SIGNING_IDENTITY="-" npm run tauri build -- --bundles dmg,app`
      succeeds (arm64; x64 if the toolchain allows).
- [ ] `hdiutil verify` the DMG; mount it and
      `codesign --verify --deep --strict` the inner `.app`.
- [ ] `shasum -a 256` → `RELEASE_CHECKSUMS.txt`.
- [ ] Owner runs the app once and confirms.
