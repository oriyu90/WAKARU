# v0.2.0 release record

Finalized 2026-09-09; **round 2 folded in 2026-09-10** (the `v0.2.0` tag was
moved to the round-2 commit on `main`; seven further owner reports — see
`docs/DECISIONS.md` D-32). Implementation plan:
`IMPLEMENTATION_PLAN_v0.2.0.md` (Round 2 section). Verification:
`QUALITY_REPORT.md`. Release body: `RELEASE_NOTES.md`.

## Verified artifact (round 2 rebuild — current)

- App: `src-tauri/target/release/bundle/macos/WAKARU.app` (arm64, version 0.2.0)
- DMG: `WAKARU_0.2.0_aarch64.dmg`
- Checksum file: `WAKARU_0.2.0_aarch64.dmg.sha256` (basename only)
- Size: `23,141,084` bytes
- SHA-256: `139ce4c03dbb8a88ea7404e562806171fc3569b51d49b1a630a1a03d55e2fffc`
- Platform: macOS 12+, Apple Silicon, ad-hoc signed, not notarized

(The 2026-09-09 build was `23,149,813` bytes / SHA-256
`415a7d2f862babd541b08c49cf98ef604be5053296731f82121a6dd385057731`; superseded.)

`hdiutil verify` VALID; `codesign --verify --deep --strict` passes for the build
output and for the app inside the mounted DMG; startup probe reached
`WAKARU backend ready version="0.2.0"`; the log contains no secret patterns.

## Release scope

- Round 2: a base URL with no path is completed to `…/v1` (LM Studio's
  "Unexpected endpoint / no reply"); a model can be picked from the server's
  `GET /models` list; Live Illustrator explains the whole document on open and
  is decoupled from Studio; the panel is rebuilt; past Studio conversations
  stay closed until opened; the panel auto-shows when the feature is on.
- LM Studio / OpenAI-compatible replies return: a stream that ends after a
  terminal `finish_reason` is complete even without `data: [DONE]`.
- Studio degrades to plain chat (retry once without tools) when the model rejects
  tool definitions.
- The streaming chat POST is not re-sent on a 5xx response (double-generation).
- 資料を見る redesigned around a vertical tab rail; an opened document fits the
  window width and re-fits on resize; Live Illustrator has a discoverable toggle.
- Studio: the composer clears on send; the tab list no longer loads every tab's
  full history; the selected tab is kept across refreshes.
- Fullscreen drops the empty traffic-light inset.
- Existing screen hierarchy (minus the redesigned viewer internals), data format,
  DB schema, design tokens and capabilities are unchanged. No project-format or
  database migration; v0.0.0–v0.1.3 projects and settings remain compatible.
  ts-rs bindings change only by the added `StudioTab.messageCount` field.

## Build

```bash
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

Signed app staged with an `/Applications` symlink and packaged as a UDZO DMG with
`hdiutil`; disk image and mounted app both verified. CMake must be on `PATH` for
the bundled `whisper.cpp` build.

## Publication sequence

1. Commit the fixes, version metadata and release documentation on `main`.
2. Tag `v0.2.0`; push the branch and the tag.
3. `gh release create v0.2.0 --repo oriyu90/WAKARU --latest` with the DMG and the
   basename-only checksum file. Repository stays private.
4. Re-download the assets and verify Latest status, byte size and hash.
5. studio-rizi: bump version strings to `0.2.0`, set the wakaru card and the four
   intro pages' hero note to **"近日公開 / Coming soon / 即将推出 / Em breve"**,
   add a 4-language NEWS entry; `npm test` + build; push; confirm the Cloudflare
   deploy. (Per the owner: the site says "coming soon" even though the DMG is
   published to GitHub the same day.)
6. Record the final commit and release details in
   `common-rules-document/WAKARU.md`.

## Explicit limits

- No Developer ID signature or Apple notarization credentials were supplied.
- Windows and Linux are not built or verified.
- With `titleBarStyle: "Overlay"` the macOS traffic lights are part of the
  webview content by design; they are positioned in the top bar, not moved
  outside the window.
