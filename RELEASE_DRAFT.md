# v0.1.2 release record

Finalized 2026-09-09. Implementation plan: `IMPLEMENTATION_PLAN_v0.1.2.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.1.2)
- DMG: `WAKARU_0.1.2_aarch64.dmg`
- Checksum file: `WAKARU_0.1.2_aarch64.dmg.sha256` (basename only)
- Size: `23,129,323` bytes
- SHA-256: `9950bc0d325a680c094a9edf4a6c67fb0c1825aca3e891bb57ae9798bc38df7d`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` is VALID. The startup probe reached
`WAKARU backend ready version="0.1.2"` and the log contains no secret patterns.

## Release scope

- Settings switches respond to a mouse click / tap again: the decorative track and
  thumb spans no longer intercept the pointer above the hidden checkbox
  (`src/components/controls.module.css`, `pointer-events: none`). Keyboard
  operation, focus ring and accessibility contract are unchanged.
- stdio MCP command resolution widened: a bare `npx` / `uvx` / `node` command is
  resolved against `$PATH` plus the usual interpreter install locations that a
  Finder-launched app does not inherit, and the server process receives that same
  widened `PATH`. Shell-free launch and the secret-free env allowlist are
  unchanged.
- "SearXNG (web search)" preset in the Add-MCP-server dialog (form-fill only).
- Existing screen hierarchy, data format, IPC, database schema, design tokens and
  capabilities are unchanged. No project-format or database migration; existing
  v0.0.0–v0.1.1 projects and settings remain compatible. ts-rs bindings unchanged.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

The signed app is staged with an `/Applications` symlink and packaged as a UDZO
DMG with `hdiutil`. Both the disk image and its mounted app are verified before
publication. CMake must be on `PATH` for the bundled `whisper.cpp` build.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag the release as `v0.1.2` and push the branch and tag.
3. Publish the GitHub release with the DMG and basename-only checksum file
   (`--repo oriyu90/WAKARU --latest`). The repository remains private.
4. Re-download the published assets and verify Latest status, byte size and hash.
5. Publish the four localized introduction pages and project/news metadata on
   `oriyu90/studio-rizi`.
6. Record final commit and release details in the maintenance repository
   (`common-rules-document/WAKARU.md`).

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- Semantic embedding search still requires an endpoint that implements
  `/v1/embeddings`; the built-in text index remains available when it does not.
- The SearXNG preset needs a running SearXNG instance with its JSON output format
  enabled, and Node.js (`npx`) available; WAKARU does not bundle either.
