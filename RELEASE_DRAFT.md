# v0.1.1 release record

Finalized 2026-09-09. Implementation plan: `IMPLEMENTATION_PLAN_v0.1.1.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.1.1)
- DMG: `WAKARU_0.1.1_aarch64.dmg`
- Checksum file: `WAKARU_0.1.1_aarch64.dmg.sha256` (basename only)
- Size: `22,662,684` bytes
- SHA-256: `232f8d4f11dc891d655ad47c401966e980a7b7b9cc8122631eb209e8c970caeb`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` is VALID. The startup probe reached
`WAKARU backend ready version="0.1.1"` and the log contains no secret patterns.

## Release scope

- Reasoning-model streaming compatibility: a leading inline `<think>…</think>`
  block is routed to the reasoning channel instead of the answer.
- Live Illustrator clears a stale question error when a new explanation starts.
- `cargo deny check` accepts `RUSTSEC-2024-0436` (`paste`, maintenance-status,
  transitive via `fastembed`) as a recorded exception with a written reason.
- Existing screen hierarchy, data format, IPC, database schema and capabilities
  are unchanged. No project-format or database migration; existing v0.0.0–v0.1.0
  projects and settings remain compatible.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

The signed app is staged with an `/Applications` symlink and packaged as a UDZO
DMG with `hdiutil`. Both the disk image and its mounted app are verified before
publication.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag the release as `v0.1.1` and push the branch and tag.
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
