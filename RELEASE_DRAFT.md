# v0.3.0 release record

Prepared 2026-09-10. Implementation plan: `IMPLEMENTATION_PLAN_v0.3.0.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.3.0)
- DMG: `WAKARU_0.3.0_aarch64.dmg`
- Checksum file: `WAKARU_0.3.0_aarch64.dmg.sha256` (basename only)
- Size: `23,163,604` bytes
- SHA-256: `61f99d360ad821eb839d98f2c17599e3ab33d7078312001c922e64fa36bf19de`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` is VALID. The mounted-DMG startup
probe reached `WAKARU backend ready version="0.3.0"` and its log contains no
secret patterns. The packaged UI was exercised for Viewer adjustments, Studio
edit/cancel and the MLXBar preset.

## Release scope

- PDF/DOCX/PPTX get view-only invert and clarity controls with bounded PDF
  sharpening and no mutation of originals or search context.
- Live follow-ups use bounded conversation-aware RAG filtered by the current
  page/source/project locator.
- Studio uses Enter to send, Shift+Enter for newline, preserves IME composition,
  and transactionally replaces a conversation tail when an earlier user turn is
  edited and resubmitted.
- MLXBar is available as a localhost preset and covered across model discovery,
  Bearer auth, reasoning/content/usage streaming and completion termination.
- No project-format, database or dependency migration; existing v0.0.0–v0.2.2
  data remains compatible.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

The signed app is staged with an `/Applications` symlink and packaged as a
UDZO DMG with `hdiutil`. Both the disk image and its mounted app are verified
before publication.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag the release as `v0.3.0` and push the branch and tag.
3. Publish the GitHub release with the DMG and basename-only checksum file
   (`--repo oriyu90/WAKARU --latest`).
4. Re-download the published assets and verify Latest status, byte size and hash.
5. Publish the four localized introduction pages and project/news metadata on
   `oriyu90/studio-rizi`.
6. Record final commit and release details in
   `common-rules-document/WAKARU.md`.

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- The live endpoint acceptance test needs an explicitly supplied endpoint and
  credentials; it remains ignored when those are absent.
