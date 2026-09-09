# v0.2.2 release record

Prepared 2026-09-10. Implementation plan: `IMPLEMENTATION_PLAN_v0.2.2.md`.
Verification: `QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.2.2)
- DMG: `WAKARU_0.2.2_aarch64.dmg`
- Checksum file: `WAKARU_0.2.2_aarch64.dmg.sha256` (basename only)
- Size: `23,156,939` bytes
- SHA-256: `1402b35ed07e5eed26593e550ecc8df083cc904dd33231e530a95f413b7bc68c`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` is VALID. The startup probe reached
`WAKARU backend ready version="0.2.2"` and the log contains no secret patterns.

## Release scope

- Studio-generated Markdown/PDF artifacts open as Viewer sources; queued-source
  cache refresh and PPTX loaded-state rendering are fixed.
- Whole-source Live Illustrator stays on one session across page navigation.
  Nothing enters Studio without the explicit handoff action.
- AI context, artifact paths, ZIP extraction, project/source identifiers and
  cross-project search-index ownership are bounded and validated.
- No project-format or database migration; existing v0.0.0–v0.2.1 projects and
  settings remain compatible. v0.2.1 website artifacts remain valid.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

The signed app is staged with an `/Applications` symlink and packaged as a UDZO
DMG with `hdiutil`. Both the disk image and its mounted app are verified before
publication.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag the release as `v0.2.2` and push the branch and tag.
3. Publish the GitHub release with the DMG and basename-only checksum file
   (`--repo oriyu90/WAKARU --latest`). The repository is private.
4. Re-download the published assets and verify Latest status, byte size and hash.
5. Publish the four localized introduction pages and project/news metadata on
   `oriyu90/studio-rizi`.
6. Record final commit and release details in the maintenance repository
   (`common-rules-document/WAKARU.md`).

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- The live endpoint acceptance test needs an explicitly supplied endpoint and
  credentials; it remains ignored when those are absent.
