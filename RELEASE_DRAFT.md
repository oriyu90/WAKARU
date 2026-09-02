# v0.0.3 release record

Finalized 2026-09-03. Implementation plan: `IMPLEMENTATION_PLAN_v0.0.3.md`
(§8 covers this release). Verification: `QUALITY_REPORT.md`. Public release body:
`RELEASE_NOTES.md`.

## Repository policy (changed in v0.0.3)

- **`oriyu90/WAKARU` is now the public source repository** (MIT, full history).
  The earlier "docs-only, never push source" policy was reversed by the owner
  (`docs/DECISIONS.md` D-21).
- `main` on `oriyu90/WAKARU` is authoritative. Commit author is
  `Yuki Orita <yukiorita0911.official@gmail.com>`.
- Releases carry the macOS Apple-Silicon DMG + its `.sha256`.
- v0.0.0 / v0.0.1 / v0.0.2 remain formal historical releases. v0.0.3 is Latest.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.0.3)
- DMG: `src-tauri/target/release/bundle/dmg/WAKARU_0.0.3_aarch64.dmg`
- Checksum file: `WAKARU_0.0.3_aarch64.dmg.sha256` (basename only)
- Size: `22,340,233` bytes
- SHA-256: `06a8c55753f0867ebb46d82c4fa9ce8ce5f6c34495ca0fa48851c36cdb0a0408`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

`codesign --verify --deep --strict` passes for the build output and for the app
inside the mounted DMG. `hdiutil verify` VALID. `NSLocalNetworkUsageDescription`
present in the bundled and DMG-mounted `Info.plist`. Startup probe reached
`WAKARU backend ready version="0.0.3"`; startup log free of secret patterns.
`spctl` rejects (expected — ad-hoc, not notarized).

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
# then, from src-tauri/target/release/bundle/ :
#   stage WAKARU.app + a /Applications symlink, hdiutil create -format UDZO,
#   hdiutil verify, shasum -a 256 > <name>.sha256  (basename only)
```

## Publication sequence

1. Rewrite commit author to `Yuki Orita <yukiorita0911.official@gmail.com>` across
   all history (fresh public repo, hashes change).
2. Delete `docs/最後にやって欲しいことと守って欲しいこと.md` from the tree (kept in
   history); it stated the now-reversed non-publication policy.
3. Commit `feat: macOS-native UI remake` and `release: WAKARU v0.0.3`; local tag `v0.0.3`.
4. Mirror-clone the current public (docs-only) repo as a local backup.
5. `git remote add origin https://github.com/oriyu90/WAKARU.git`;
   `git push --force origin main`; `git push origin v0.0.3`.
6. Update the repo description / homepage (drop "source stays private").
7. `gh release create v0.0.3 --repo oriyu90/WAKARU --latest --title "WAKARU v0.0.3"
   --notes-file RELEASE_NOTES.md <dmg> <dmg.sha256>`.
8. Update Studio RIZI's four WAKARU pages + `website/content.js`; run
   `npm test && npm run build && npm run validate && npm run count-files`; push.
9. Update `common-rules-document/WAKARU.md` and this file with the final artifact
   and commits.
10. Re-download the published DMG and confirm name, byte size, SHA-256,
    non-draft/non-prerelease and Latest.

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- All v0.0.2 feature limits still apply (legacy binary Office, encrypted/corrupt
  documents, unsupported media codecs, scanned-PDF OCR indexing).
