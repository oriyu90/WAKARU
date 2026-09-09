# v0.1.3 release record

Finalized 2026-09-09. Implementation plan: `IMPLEMENTATION_PLAN_v0.1.3.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.1.3)
- DMG: `WAKARU_0.1.3_aarch64.dmg`
- Checksum file: `WAKARU_0.1.3_aarch64.dmg.sha256` (basename only)
- Size: `23,129,085` bytes
- SHA-256: `657c9613d8a7d3d180f440e9421556b6f199b8cef2b3baa50e0724bcc4c1e4ca`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` is VALID. The startup probe reached
`WAKARU backend ready version="0.1.3"` and the log contains no secret patterns.

## Release scope

- The Settings switches respond to a click in the packaged app: `Switch` is now a
  `<label>`, which forwards a press anywhere on the control to the input in every
  engine (v0.1.2 fixed only Chromium; the app ships WKWebView). Keyboard, focus
  ring and accessible name unchanged (`src/components/Switch.tsx`).
- Right-click a project in the sidebar for an Export (ZIP) / Delete menu — a new
  `ContextMenu` component wired in `AppShell`, using the same IPC as Settings →
  Project Management. Delete asks for confirmation.
- The Settings button is a toggle: pressing it on `/settings` returns to the view
  you came from.
- The "資料を見る" pane fills its width instead of collapsing to the left
  (`Viewer.module.css` `.pane > * { flex:1; min-width:0 }`).
- A Live Illustrator conversation is kept per document, not per page, so it
  survives a page turn (`IllustratorDrawer` thread key; page explanations still
  per page). Studio keeps the selected tab across refetches and no longer
  double-renders a streamed reply.
- Existing screen hierarchy, data format, IPC, database schema, design tokens and
  capabilities are unchanged. No project-format or database migration; existing
  v0.0.0–v0.1.2 projects and settings remain compatible. ts-rs bindings unchanged.
  No Rust source change.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

The signed app is staged with an `/Applications` symlink and packaged as a UDZO
DMG with `hdiutil`. Both the disk image and its mounted app are verified before
publication. CMake must be on `PATH` for the bundled `whisper.cpp` build.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag the release as `v0.1.3` and push the branch and tag.
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
- Live Illustrator question threads created before this release are not deleted,
  but are no longer surfaced in the drawer (one thread per document now).
- Semantic embedding search still requires an endpoint that implements
  `/v1/embeddings`; the built-in text index remains available when it does not.
