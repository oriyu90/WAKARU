# v0.0.5 release record

Finalized 2026-09-07. Implementation plan: `IMPLEMENTATION_PLAN_UI_REMAKE.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app`
  (arm64, version 0.0.5)
- DMG: `src-tauri/target/release/bundle/dmg/WAKARU_0.0.5_aarch64.dmg`
- Checksum file: `WAKARU_0.0.5_aarch64.dmg.sha256` (basename only)
- Size: `23,079,059` bytes
- SHA-256: `66d0ea04f3f10673b87b74d2072a06a1b09ed7399c5d9b2c7540ba803b133c19`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` is VALID. The startup probe reached
`WAKARU backend ready version="0.0.5"` and the log contains no secret patterns.

## Release scope

- UI-only refresh covering the application shell and all main workspaces.
- Existing screen hierarchy and capabilities remain in place.
- No IPC, project format or database migration change.
- Existing v0.0.0–v0.0.4 projects and settings remain compatible.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

The signed app is staged with an `/Applications` symlink and packaged as a UDZO
DMG with `hdiutil`. Both the disk image and its mounted app are verified before
publication.

## Publication sequence

1. Commit the UI refresh, version metadata and release documentation on `main`.
2. Tag the release as `v0.0.5` and push the branch and tag.
3. Publish the GitHub release with the DMG and basename-only checksum file.
4. Re-download the published assets and verify Latest status, byte size and hash.
5. Publish the four localized introduction pages and project/news metadata.
6. Record final commit and release details in the maintenance repository.

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- Functional capabilities and their prior limits are unchanged from v0.0.4.
