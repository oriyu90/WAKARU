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
