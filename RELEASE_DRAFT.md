# v0.0.2 release record

Finalized 2026-09-02. The authoritative implementation plan is
`IMPLEMENTATION_PLAN_v0.0.2.md`; verification is in `QUALITY_REPORT.md`; the
public release body is `RELEASE_NOTES.md`.

## Repository policy

- The development repository has no remote by design. Do not push application
  source to the public `oriyu90/WAKARU` repository.
- The public repository contains distribution documentation, license inventory,
  website files and binary release assets only.
- v0.0.0 and v0.0.1 remain formal historical releases. v0.0.2 becomes Latest.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app`
- DMG: `src-tauri/target/release/bundle/dmg/WAKARU_0.0.2_aarch64.dmg`
- Checksum file: `WAKARU_0.0.2_aarch64.dmg.sha256`
- Size: `21,391,026` bytes
- SHA-256: `b46ef031b81578db4bdcccf01da7210f5b9091789c74c92a1704be6a2614a225`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

The optimized app, mounted-DMG signature, six-second startup probe, backend
version, secret-free startup log and DMG checksum all passed.

## Publication sequence

1. Commit and locally tag the private source tree as `v0.0.2`; do not push it.
2. Copy README, release notes, quality report and third-party licenses to the
   public distribution repository; update its four-language static page.
3. Update Studio RIZI's four WAKARU pages and central update entry; preserve the
   unrelated `.hallmark/` directory.
4. Update `common-rules-document/WAKARU.md` with the final artifact and commits.
5. Push public documentation and website commits.
6. Create formal GitHub Release `v0.0.2`, attach the DMG and checksum, use
   `RELEASE_NOTES.md`, and mark it Latest.
7. Re-download or query both published assets and confirm names, byte size,
   checksum, non-draft/non-prerelease state and Latest redirect.

## User installation

Download `WAKARU_0.0.2_aarch64.dmg` from the v0.0.2 release, verify its SHA-256,
open the DMG and drag WAKARU to Applications. Because this build is ad-hoc signed
and not notarized, first launch may require Finder → right-click WAKARU → Open.

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- Scanned PDFs render visually but pages without a text layer are not
  automatically OCR-indexed.
- Encrypted/corrupt documents and unsupported legacy media codecs fail with a
  recoverable error or use extracted-text fallback.
