# WAKARU v0.1.2 implementation and verification plan

Date: 2026-09-09

## 1. Objective

Fix two owner-reported problems and release v0.1.2. No schema, IPC-contract,
ts-rs-binding, or screen-hierarchy changes. Preserve compatibility, existing
features, crash safety, memory safety, and complete Japanese / English / Simplified
Chinese UI parity. The repository stays private.

1. **"Live Illustrator cannot be enabled."** The Settings toggle does nothing on
   click.
2. **SearXNG in an MCP server** — determine whether it can be configured today,
   and if not, make it possible.

## 2. Diagnosis

### P1 — Settings switches are click-dead (severity: medium — a shipped control does nothing on click)

`src/components/Switch.tsx` renders a visually hidden `<input type="checkbox">`
followed by two decorative spans, `.switchTrack` and `.switchThumb`, all
`position: absolute`. The spans have the default `pointer-events: auto` and are
painted after the input, so `document.elementFromPoint` at the centre of the
control returns `.switchTrack`, not the input. A mouse click or touch tap never
reaches the checkbox; only keyboard (focus + Space) toggles it. Verified in the
running app.

Every `<Switch>` in Settings is affected — Enable Live Illustrator, Check for
updates on startup, OCR, Prefetch next page, Carry over the previous page's
conversation. `Checkbox` is unaffected because it wraps its input in a `<label>`.

The setting's persistence (`useUiStore` → localStorage), the backend merge
(`services/settings.rs::deep_merge`), and the Viewer's `illustratorEnabled`
gating are all correct. Only the pointer target is broken.

### P2 — SearXNG MCP server is representable but not runnable (severity: medium — a documented capability is unreachable in practice)

The MCP layer accepts a generic stdio server (`command` + `args` + `env`, env
values keychained), so `npx -y mcp-searxng` with `SEARXNG_URL` *can be entered*.
It fails at connect time: a macOS `.app` launched from Finder inherits only
`PATH=/usr/bin:/bin:/usr/sbin:/sbin`, and `services/mcp.rs::connect()` does
`env_clear()` then re-adds only that minimal `PATH`, so `npx` / `uvx` / `node` do
not resolve → `MCP_SPAWN_FAILED`. There is also no preset or guidance.

## 3. Fix plan

### F1 → P1: stop the decoration from eating the click

`src/components/controls.module.css`: add `pointer-events: none` to `.switchTrack`
and `.switchThumb`. Standard pattern for a decorative overlay above a visually
hidden input. No behaviour, layout, focus-ring, or a11y change; keyboard
unaffected.

Regression test `src/components/Switch.test.tsx`: the switch exposes the `switch`
role and an accessible name; a `userEvent.click` on the control toggles a
controlled `checked`; Space still toggles.

### F2 → P2: resolve stdio commands from the usual locations, and ship a SearXNG preset

`src-tauri/src/services/mcp.rs`:

- `extra_bin_dirs()` — a fixed list of interpreter/tool-manager `bin` directories
  a GUI process misses (`/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`,
  `~/.cargo/bin`, `~/.bun/bin`, `~/.deno/bin`, `~/.volta/bin`, `~/Library/pnpm`,
  `/opt/local/bin`, and every `.nvm` / `fnm` node-version `bin`).
- `child_path()` — inherited `PATH` first, then `extra_bin_dirs()`, de-duplicated,
  existing directories only.
- `resolve_program(program, search_path)` — a `/`-qualified or absolute command is
  returned unchanged; otherwise the first executable match on `search_path`;
  otherwise the original name (so `spawn` fails exactly as before).
- `connect()` uses `resolve_program` for the executable and sets the child's
  `PATH` to `child_path()`. `env_clear()`, the shell-free launch, the
  PATH/HOME/TMPDIR/LANG allowlist, and per-server env (which can still override
  `PATH`) are unchanged. Purely additive to lookup order.

Unit tests: `child_path` is deduped and every entry exists; `resolve_program`
finds `sh` as an absolute path; absolute / relative / unknown names pass through
verbatim.

`src/features/settings/McpSettings.tsx`: in the Add-server dialog (new servers
only), a preset `<Select>` — "Custom" (default) and "SearXNG (web search)".
Choosing SearXNG fills the draft with `npx` / `-y mcp-searxng` /
`SEARXNG_URL=http://localhost:8888`. The user reviews and saves; nothing
connects. No new IPC, no backend change.

`src/i18n/{ja,en,zh-Hans}.json`: new `mcp.preset`, `mcp.presetCustom`,
`mcp.presetSearxng`, `mcp.presetSearxngHint`; `mcp.commandHint` updated to mention
automatic `npx` / `uvx` / `node` resolution. Exact key parity across all three
languages.

### Docs (last, per common rules §1)

`docs/DECISIONS.md` D-29; `docs/HANDOFF.md` v0.1.2 note; `README.md` MCP line and
build requirements (CMake); `RELEASE_NOTES.md` / `QUALITY_REPORT.md` (newest on
top) / `RELEASE_DRAFT.md`; `THIRD_PARTY_LICENSES.md` regenerated (no dependency
change expected).

## 4. Verification

- Full FE gate: `typecheck`, `lint` (eslint + design-rules + hardcoded-strings),
  `test` (vitest incl. the new Switch cases), `check:contrast`, `check:i18n`,
  `build`.
- `npm run bindings` — ts-rs drift 0.
- Full Rust gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` (lib incl. the 3 new MCP path cases + integration), `cargo deny
  check`.
- In the running app (`npm run dev`): every Settings switch toggles on click in
  Japanese and English, light and dark, and persists across reload; Enable Live
  Illustrator turns on and the Viewer handle appears. The MCP SearXNG preset fills
  every field.
- `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build --
  --bundles app`; ad-hoc sign; standard UDZO DMG via `hdiutil`; `hdiutil verify`;
  `codesign --verify --deep --strict` on the app inside the mounted DMG;
  `Info.plist` still declares `NSLocalNetworkUsageDescription`; startup probe
  reaches `backend ready version="0.1.2"`; startup log has no secret patterns;
  `shasum -a 256 > WAKARU_0.1.2_aarch64.dmg.sha256` (basename only).

## 5. Release

Version in 5 files (`package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
`src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`) → `0.1.2`. Commit as
`Yuki Orita`, push `origin main` + tag `v0.1.2`.
`gh release create v0.1.2 --repo oriyu90/WAKARU --latest --notes-file RELEASE_NOTES.md WAKARU_0.1.2_aarch64.dmg WAKARU_0.1.2_aarch64.dmg.sha256`;
re-download the asset and re-check the checksum.
`oriyu90/studio-rizi` `website/projects/wakaru/` (4 languages) + `website/content.js`
(`releaseVersion` / `releaseDate` + one UPDATE entry ×4 languages);
`npm test && npm run build && npm run validate && npm run count-files`; push `main`.
`common-rules-document/WAKARU.md`: add the v0.1.2 entry and update "current
release". Repository remains **private**.
