# v0.2.1 release record

Cut 2026-09-10. Five further owner reports (`docs/DECISIONS.md` D-33).
Implementation plan: `IMPLEMENTATION_PLAN_v0.2.1.md`. Verification:
`QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.2.1)
- DMG: `WAKARU_0.2.1_aarch64.dmg`
- Checksum file: `WAKARU_0.2.1_aarch64.dmg.sha256` (basename only)
- Size: 23,178,061 bytes
- SHA-256: `6e3218f73da5814e795266e3622adf9c5b46362ec7f7364eadc27379ffb26283`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized
- `hdiutil verify` VALID; inner-app `codesign --deep --strict` passes on the
  mounted DMG; startup probe reached `WAKARU backend ready version="0.2.1"`; log
  free of secret patterns.

## Release scope

- Live Illustrator: the 資料全体／このページ and detail-level segmented controls
  are removed; the panel always explains the whole document at the Settings
  detail level, and the other two levels are one-tap rewrites shown under the
  first automatic explanation (only before the reader's first follow-up).
- 資料を見る: `←` / `→` / `PageUp` / `PageDown` turn PDF and slide pages, hover
  `‹ ›` arrows sit on the page, and Word / Markdown / web previews get an
  up/down scroll control.
- Studio: a tool result is a collapsed block you expand (like Claude).
- Studio: a new `build_site` tool authors a multi-file static website as one
  folder artifact; PDF and Word (`build_document`) are unchanged.
- 資料を見る: **Add website** imports a static-site folder (bounded, symlink-safe
  copy), recognises `index.html`, extracts each HTML file's text, and previews
  the site in a sandboxed frame served from the project's asset scheme.
- Existing screen hierarchy, data format, DB schema, design tokens and
  capabilities are unchanged. **No project-format or database migration**;
  v0.0.0–v0.2.0 projects and settings remain compatible. ts-rs bindings change
  only by `WebsiteFile` / `WebsiteManifest` and a `"website"` `SourceKind`
  member.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

Signed app staged with an `/Applications` symlink and packaged as a UDZO DMG with
`hdiutil`; disk image and mounted app both verified. CMake must be on `PATH` for
the bundled `whisper.cpp` build.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag `v0.2.1`; push the branch and the tag.
3. `gh release create v0.2.1 --repo oriyu90/WAKARU --latest` with the DMG and the
   basename-only checksum file. Repository stays private.
4. Re-download the assets and verify Latest status, byte size and hash.
5. studio-rizi: keep **"近日公開 / Coming soon"** for the wakaru card and the
   intro pages; the download link stays at `releases/tag/v0.1.3`. Add a
   4-language NEWS entry; `npm test` + build; push; confirm the Cloudflare
   deploy.
6. Record the final commit and release details in
   `common-rules-document/WAKARU.md`.

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- The website preview frame runs scripts (`sandbox="allow-scripts
  allow-same-origin allow-forms"`); imported sites share one local asset origin.
