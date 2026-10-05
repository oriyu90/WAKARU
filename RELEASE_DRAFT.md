# v1.6.9 release record

Prepared 2026-10-05. Scope: sticky-note stabilisation on document coordinates
(D-49–D-57) plus audit fixes (D-58). See `RELEASE_NOTES.md`,
`docs/DECISIONS.md` D-49–D-58 and `QUALITY_REPORT.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, macOS 12+,
  version 1.6.9, ad-hoc signed with runtime option, identifier
  `com.yukiorita.wakaru`; strict verify passes after manual re-sign due to the
  known linker-signed bundler skip)
- DMG: `WAKARU_1.6.9_aarch64.dmg` (21,958,941 bytes)
- Checksum: `WAKARU_1.6.9_aarch64.dmg.sha256`
- SHA-256: `9701db2cb2aa10308c33b2efa458f871393ef351eef208d04b78d33d430a2daf`
- `hdiutil verify` was VALID. The built app reached
  `WAKARU backend ready version="1.6.9"`.
- No credentials are embedded in tracked source, new documentation or DMG.

## Verification summary

- Frontend: typecheck, lint (0 warnings), design-rules, hardcoded-strings,
  57 tests, contrast, i18n 487 keys × 3, production build.
- Rust: fmt, clippy all-targets/all-features `-D warnings`, 236 lib passed +
  1 ignored, all integration phases, `cargo deny check` (advisories/bans/
  licenses/sources).
- No DB migration, no IPC change (additive-only since v1.6.0). v1.6.0–v1.6.1
  projects, notes and figures open as-is; pre-v1.6.4 note coordinates show the
  position-check badge.

## Steps

1. Commit and tag `v1.6.9` on `main`.
2. Create the GitHub Release `v1.6.9` with the DMG + `.sha256`.
3. Update the Studio RIZI site (4 locales) and the private `WAKARU.md` memo.

---

# v1.6.1 release record

Prepared 2026-10-01. Scope: v1.6.0 audit fixes for notes and figures.
See `RELEASE_NOTES.md`, `docs/DECISIONS.md` D-48 and `QUALITY_REPORT.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, macOS 12+,
  version 1.6.1, ad-hoc signed with runtime option)
- DMG: `WAKARU_1.6.1_aarch64.dmg` (23,344,079 bytes)
- Checksum: `WAKARU_1.6.1_aarch64.dmg.sha256`
- SHA-256: `fcece04b1f051b0fa032764e66ac5a6b5b081a81d6935acc62fecee187c11307`
- `codesign --verify --deep --strict` passed on the built app and mounted DMG
  app. `hdiutil verify` was VALID. The mounted app reached
  `WAKARU backend ready version="1.6.1"`.
- No credentials are embedded in tracked source, new documentation or DMG.

## Verification summary

- Frontend: typecheck, lint/design/hardcoded checks, 45 tests, contrast,
  i18n parity (486 keys × 3 languages), production build and production npm
  audit (0 vulnerabilities) passed.
- Rust: fmt, clippy with denied warnings, 236 library tests passed (1 ignored),
  phase integration suites and cargo-deny passed. No database migration.
- Audit coverage: stack-level creation listener with overlay-chrome guard,
  lane pointer discipline, editor separation, conflict-clear on save, figure
  stop/timeout/retry, aspect fallback (both sides), PDF Range preflight,
  windowed paging continuity and external page-jump follow.

## Publication sequence

1. Commit and tag `v1.6.1` on `main`.
2. Publish the GitHub release with the verified DMG and checksum; re-download
   the assets and compare hash and size.
3. Publish four localized Studio RIZI pages and release/news metadata after
   the site tests.
4. Record the release in the private common-rules maintenance document.

## Limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- The GitHub repository remains private.

---

# v1.6.0 release record

Prepared 2026-10-01. Scope: large-source staging, interactive figures, sticky
notes. See `RELEASE_NOTES.md`, `DESIGN_v1.6.0.md` and `QUALITY_REPORT.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, macOS 12+,
  version 1.6.0, ad-hoc signed with runtime option)
- DMG: `WAKARU_1.6.0_aarch64.dmg` (23,341,800 bytes)
- Checksum: `WAKARU_1.6.0_aarch64.dmg.sha256`
- SHA-256: `abc2ff797e5c1ae3416e0556fccf8a3f256267703111bdfaacc558f52d69d56c`
- `codesign --verify --deep --strict` passed on the built app and mounted DMG
  app. `hdiutil verify` was VALID. The mounted app reached
  `WAKARU backend ready version="1.6.0"`.
- No credentials are embedded in tracked source, new documentation or DMG.

## Verification summary

- Frontend: typecheck, lint/design/hardcoded checks, 42 tests, contrast,
  i18n parity (485 keys × 3 languages), production build and production npm
  audit (0 vulnerabilities) passed.
- Rust: fmt, clippy with denied warnings, 235 library tests passed (1 ignored),
  all phase integration suites, bindings (+additive) and cargo-deny passed.
- Migration: pre-1.6.0 project DB migrates forward; notes CRUD/conflict/
  delete/restore, visual validation blocklist, windowed text read with
  absolute line numbers, and old-ZIP-as-zero import covered by tests.
- Live real-model acceptance (owner LAN) remains for owner-side confirmation;
  the Live figure path also renders a deterministic offline SVG summary.

## Publication sequence

1. Commit and tag `v1.6.0` on `main`.
2. Publish the GitHub release with the verified DMG and checksum; re-download
   the assets and compare hash and size.
3. Publish four localized Studio RIZI pages and release/news metadata after
   the site tests.
4. Record the release in the private common-rules maintenance document.

## Limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- The GitHub repository remains private.

---

# v1.5.0 release record

Prepared 2026-10-01. Scope: Studio send persistence reconciliation, tool
validation with finite recovery, and whole-document translation PDF.
See `RELEASE_NOTES.md`, `DESIGN_v1.5.0.md` and `QUALITY_REPORT.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, macOS 12+,
  version 1.5.0, ad-hoc signed with runtime option)
- DMG: `WAKARU_1.5.0_aarch64.dmg` (23,253,659 bytes)
- Checksum: `WAKARU_1.5.0_aarch64.dmg.sha256`
- SHA-256: `eca4749c566c9380cfbfd5a4478d94c03a8314d04232c5a4b7d72c85cc0d0bae`
- `codesign --verify --deep --strict` passed on the built app and mounted DMG
  app. `hdiutil verify` was VALID. The mounted app reached
  `WAKARU backend ready version="1.5.0"`.
- No credentials are embedded in tracked source, new documentation or DMG.

The Tauri bundler skipped a complete app signature. The app was ad-hoc signed
with identifier `com.yukiorita.wakaru`, then the DMG was rebuilt from the
signed app (convert to UDRW, replace app, convert back to UDZO) before image
and mounted-app verification.

## Verification summary

- Frontend: typecheck, lint/design/hardcoded checks, 38 tests, contrast,
  i18n parity (433 keys × 3 languages), production build and production npm
  audit (0 vulnerabilities) passed.
- Rust: fmt, clippy with denied warnings, 225 library tests passed (1 ignored),
  nonignored integration suites, bindings and cargo-deny passed.
- Translation PDF: 9-page mock translation renders verified `%PDF-` artifact
  in order; over-limit/empty/render failures report without partial artifacts.
- Live real-model acceptance (owner LAN) remains for owner-side confirmation;
  automated mock translation covers the new path.

## Publication sequence

1. Commit and tag `v1.5.0` on `main`.
2. Publish the GitHub release with the verified DMG and checksum; re-download
   the assets and compare hash and size.
3. Publish four localized Studio RIZI pages and release/news metadata after
   the site tests.
4. Record the release in the private common-rules maintenance document.

## Limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- The GitHub repository remains private.

---

# v1.4.0 release record

Prepared 2026-09-30. Scope: generation-aware MLXBar connection checking,
document zoom and full-width reading, Live drawer width return, and stronger
source-scoped Studio/Live retrieval and citations. See `RELEASE_NOTES.md`,
`DESIGN_v1.4.0.md`, D-45 and `QUALITY_REPORT.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, macOS 12+,
  version 1.4.0, ad-hoc signed with runtime option)
- DMG: `WAKARU_1.4.0_aarch64.dmg` (21,821,067 bytes)
- Checksum: `WAKARU_1.4.0_aarch64.dmg.sha256`
- SHA-256: `b38dbd4e24911d68838ad29ddfc6ddf452801bb256d17ff0fc336376cea4b287`
- `codesign --verify --deep --strict` passed on the built app and mounted DMG
  app. `hdiutil verify` was VALID. The mounted app reached
  `WAKARU backend ready version="1.4.0"`.
- The supplied PIN is absent from tracked source, new documentation and DMG.

The Tauri bundler skipped a complete app signature. The app was ad-hoc signed
with identifier `com.yukiorita.wakaru` and the DMG rebuilt using Tauri's
`bundle_dmg.sh` before final verification.

## Verification summary

- Owner-specified loaded MLXBar model: authenticated generation, tools, vision,
  JSON Schema, Japanese/English source explanations and a grounded Studio
  Markdown artifact passed. The unavailable embeddings route fell back to
  local retrieval. See `docs/LIVE_ORNITH_VALIDATION.md`.
- Frontend: typecheck, lint/design/hardcoded checks, 36 tests, contrast,
  i18n parity (421 keys × 3 languages), production build and production npm
  audit (0 vulnerabilities) passed.
- Rust: fmt, clippy with denied warnings, 212 library tests passed (1 ignored),
  nonignored integration suites, bindings and cargo-deny passed.
- Debug app visual inspection covered PDF zoom, Live open/close width and
  full-width document/restore.

## Publication sequence

1. Commit and tag `v1.4.0` on `main`.
2. Publish the GitHub release with the verified DMG and checksum; re-download
   the assets and compare hash and size.
3. Publish four localized Studio RIZI pages and release/news metadata after
   the site tests.
4. Record the release in the private common-rules maintenance document.

## Limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- The GitHub repository remains private.

---

# v1.2.0 release record

Prepared 2026-09-25. Scope: opt-in JavaScript live view for weblink previews
(reader/live switch, opaque-origin `allow-scripts` sandbox, CSP `frame-src`
extended to `https:`/`http:`, reader view stays default, live state resets
per document, reader fetch paused while live). Verification: this file,
`RELEASE_NOTES.md` (v1.2.0 section), `docs/DECISIONS.md` D-43.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 1.2.0)
- DMG: `WAKARU_1.2.0_aarch64.dmg`
- Checksum: `WAKARU_1.2.0_aarch64.dmg.sha256` (basename only)
- Size: `21,786,658` bytes
- SHA-256: `1f47fd74b58f92e5e23a78d6a2513416ccc2e38662ecc701a8d965d42be1a6c7`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarised

Scrutiny fixes over the first implementation: live state resets when the
document changes (no inherited remote scripts), and reader-text fetching is
disabled while the live frame is shown. The Tauri bundler again skipped
re-signing the linker-signed binary, so the bundle was finished with
`codesign --force --sign - --identifier com.yukiorita.wakaru --options
runtime` and the DMG rebuilt with the same `bundle_dmg.sh` arguments.

`codesign --verify --deep --strict` passes for the build output and the app
inside the mounted DMG. `hdiutil verify` is VALID. The mounted app reached
`WAKARU backend ready version="1.2.0"`; the startup lines contain no credential
patterns.

## Release scope

- Weblink reader/live switch (originalUrl-gated, http(s) only).
- No database migration, project archive change, AI profile migration or
  dependency change.

## Verification summary

- Frontend: typecheck, eslint, design-rules, hardcoded-strings, 33 tests,
  contrast, i18n parity, production build and npm audit (0) pass.
- Rust: unchanged since v1.1.0 gates; fmt, clippy with warnings denied and
  library tests re-run green.
- Bindings drift: none.

## Publication sequence

1. Commit and tag `v1.2.0` on `main`.
2. Publish the GitHub release with the verified DMG and checksum.
3. Re-download release assets and compare byte size and SHA-256.
4. Publish four localized Studio RIZI pages and release/news metadata.
5. Record the release in the private common-rules maintenance document.

## Explicit limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- Repository visibility remains private pending an explicit owner decision.

---

# v1.1.0 release record

Prepared 2026-09-24. Scope: Live Illustrator five-item rework (selectable
answers, past/session split, message-format chat, resizable 24–44 rem panel,
on-demand citations), rustls 0.23.43 → 0.23.45 (RUSTSEC-2026-0285).
Verification: this file, `RELEASE_NOTES.md` (v1.1.0 section).

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 1.1.0)
- DMG: `WAKARU_1.1.0_aarch64.dmg`
- Checksum: `WAKARU_1.1.0_aarch64.dmg.sha256` (basename only)
- Size: `21,785,197` bytes
- SHA-256: `f0c7c5dccec8ec74efc6ead42f4bc4145e3dba15b1ba6bbef28eb7212187990d`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarised

Build note: the Tauri bundler left the app linker-signed (identifier
`wakaru-<hash>`, strict deep verification fails), so the bundle was finished
with `codesign --force --sign - --identifier com.yukiorita.wakaru --options
runtime`, matching the v1.0.0 signature shape (`adhoc,runtime`,
`Info.plist entries=16`). DMG was rebuilt from the signed app with the same
`bundle_dmg.sh` arguments Tauri uses.

`codesign --verify --deep --strict` passes for the build output and the app
inside the mounted DMG. `hdiutil verify` is VALID. The mounted app reached
`WAKARU backend ready version="1.1.0"`; the startup lines contain no credential
patterns. Packaged-app UI inspection was replaced by component-level review
and the full gate suite (below) because no authorised AI endpoint was
available for live model runs.

## Release scope

- Live answers are selectable (drag + per-message copy); auto-scroll never
  steals a selection.
- Past turns stay collapsed; this session's turns stay expanded; rewrite
  buttons consider this session only.
- Live chat uses the Studio message contract (user bubble / assistant canvas
  + role label); citations appear only when resolved.
- Live panel resizes 24–44 rem via edge handle (pointer + keyboard),
  persisted in localStorage; document pane narrows and PDF refits.
- Citations are conditional; small-talk skips retrieval; Live topK 12 → 6.
- No database migration, project archive change, AI profile migration or
  dependency addition (rustls patch only).

## Verification summary

- Frontend: typecheck, eslint, design-rules, hardcoded-strings, 27 tests,
  contrast, 399-key i18n parity, production build and npm audit (0) pass.
- Rust: fmt, clippy --all-targets with warnings denied, 202 library tests
  (incl. 5 new illustrator tests) plus all phase integration tests pass;
  live_ornith remains intentionally ignored; cargo-deny all pass.
- Bindings drift: none (whitespace-only regeneration).

## Publication sequence

1. Commit and tag `v1.1.0` on `main`.
2. Publish the GitHub release with the verified DMG and checksum.
3. Re-download release assets and compare byte size and SHA-256.
4. Publish four localized Studio RIZI pages and release/news metadata.
5. Record the release in the private common-rules maintenance document.

## Explicit limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- Repository visibility remains private pending an explicit owner decision.

---

# v1.0.0 release record

Prepared 2026-09-11. Audit and plan: `FORMAL_RELEASE_AUDIT_AND_PLAN.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 1.0.0)
- DMG: `WAKARU_1.0.0_aarch64.dmg`
- Checksum: `WAKARU_1.0.0_aarch64.dmg.sha256` (basename only)
- Size: `23,163,028` bytes
- SHA-256: `30fdc74120e2ce48782ac1b1111a36b7f00869b51392b4bcc569e9606adb990b`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarised

`codesign --verify --deep --strict` passes for both the build output and the
app inside the mounted DMG. `hdiutil verify` is VALID. The mounted app reached
`WAKARU backend ready version="1.0.0"`; the startup lines contain no credential
patterns. Packaged-app UI inspection covered PDF correction controls and the
non-collapsing toolbar with Live Illustrator open.

## Release scope

- Studio document format, filename and MIME now agree; complete files are
  atomically persisted before artifact metadata is recorded.
- OpenAI-compatible endpoints report malformed SSE and empty completed turns
  instead of silently producing a truncated or blank answer.
- Imported websites execute in an opaque-origin preview without form or
  same-origin permission.
- PDF controls stay readable beside Live Illustrator; interactive PDF, DOCX
  and PPTX rendering uses a safer 96 MiB input ceiling.
- The four-language product page is rebuilt as a responsive Hallmark-style
  product workbench with truthful private/ad-hoc distribution messaging.
- Existing projects and conversations remain compatible; there is no database
  or archive-format migration.

## Verification summary

- Frontend: typecheck, lint/design/hardcoded, 23 tests, contrast, 395-key i18n
  parity, production build and npm production audit all pass.
- Rust: fmt, clippy with warnings denied, 197 library tests plus all phase
  integration tests pass; one network-dependent test remains intentionally
  ignored; cargo-deny advisories/bans/licenses/sources all pass.
- External Live acceptance was not run because no authorised endpoint or
  credential was supplied; mock HTTP/SSE contract coverage passed.

## Publication sequence

1. Commit and tag `v1.0.0` on `main`.
2. Publish the GitHub release with the verified DMG and checksum.
3. Re-download release assets and compare byte size and SHA-256.
4. Publish four localized Studio RIZI pages and release/news metadata.
5. Record the release in the private common-rules maintenance document.

## Explicit limits

- No Developer ID signature or Apple notarisation credentials were supplied.
- Windows and Linux are not built or verified.
- Repository visibility remains private pending an explicit owner decision.
