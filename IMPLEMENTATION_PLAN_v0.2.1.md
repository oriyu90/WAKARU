# WAKARU v0.2.1 — implementation plan

Follow-up to v0.2.0. Five reader-reported issues, an OpenAI/Anthropic wire
re-check, and the dangerous-design items alongside. Safe-design contract holds:
no project-format change, no database migration, JA / EN / zh-Hans string
parity, crash- and memory-safety, every existing capability kept. Decision
record: `docs/DECISIONS.md` D-33.

studio-rizi keeps **"近日公開 / Coming soon"** for the WAKARU project card and the
intro page; the download link stays at v0.1.3. The DMG is still built and
published to GitHub the same as prior releases — "coming soon" is only the site
wording the owner asked for.

| # | Issue | Approach |
|---|-------|----------|
| 1 | Remove the 資料全体／このページ and かんたん／標準／くわしい segmented controls from Live Illustrator; offer the other two detail levels as one-tap rewrites *below* the auto explanation, only before the first follow-up | `IllustratorDrawer.tsx`: drop `view`/`View` and both `Tabs`; always `locator {t:"whole"}`; default level from `settings.illustrator.defaultLevel`; `levelOverride` state + `.levelSwap` buttons gated on `pastMessages.length === 0 && !streaming && explanation`. i18n: `illustrator.rewriteAs`; remove `viewLabel/viewOverview/viewPage/willExplain` |
| 2 | Arrow buttons / keys move PDF & document pages or scroll | `FilePreviews.tsx`: `usePageKeys` (arrows/PageUp/PageDown, ignore typing) + `EdgeNav` hover arrows on PDF & PPTX; `ScrollNav` (buttons + PageUp/PageDown) on `DocxFilePreview` and `ReadingPreview`; typing-guard added to `PagedPreview`. i18n: `viewer.scrollUp/scrollDown` |
| 3 | Collapse Studio tool results like Claude | `Studio.tsx` `MessageRow`: `role === "tool"` → closed `<details>` with a line-count summary. i18n: `studio.toolResultLines` |
| 4 | Studio can author PDF / Word / websites | PDF & Word already exist (`build_document`). New `build_site` tool → one directory artifact (`text/x-wakaru-site`); validation before write (`.html` required, ≤200 files / 24 MB, `website::safe_rel`); `studio_import_artifact_as_source` branches to `sources::add_folder` for directory artifacts; `studio_download_artifact` copies dirs recursively. i18n: `errors.STUDIO_SITE_INVALID` |
| 5 | Import a website folder in 資料を見る and preview `index.html` | New `SourceKind::Website`; `services/website.rs` (`copy_site_tree` bounded iterative walk, no symlink follow, ext allowlist; `parse_site` one unit/HTML; `manifest`); `sources::add_folder`; ingest `Website` arm; commands `source_add_folder` / `website_manifest`; frontend `pickFolder()`, Viewer "Add website", `WebsitePreview.tsx` (file tree + sandboxed iframe via `wakaru-asset://`); CSP `frame-src`. i18n: `sourceKind.website`, `errors.WEBSITE_INVALID`, `viewer.addWebsite/websiteFiles/websiteReload/websiteOpenExternal/websitePreview/websiteEntry` |

## Preview isolation (owner's choice)

Sandboxed iframe with scripts enabled: `sandbox="allow-scripts allow-same-origin
allow-forms"`, `referrerpolicy=no-referrer`, no `allow-popups`. Served through the
in-app-only `wakaru-asset://` scheme, so the frame has an opaque origin and
cannot reach `tauri://` / `ipc:`. Residual: imported sites share the one
`wakaru-asset://localhost` host, so one could read another's files by path — all
local, user-supplied; noted in D-33.

## Dangerous-design items folded in

- Folder copy: iterative (non-recursive) walk, symlinks never followed, file
  count / total bytes / depth ceilings — bounded memory, disk and stack.
- `build_site`: whole file set validated before any write.
- `parse_site`: per-file body-extraction cap (3 MiB).
- Directory-artifact download skips symlinks, recurses a size-capped tree.

## Unchanged

IPC contract (only `source_add_folder` / `website_manifest` added; ts-rs adds
`WebsiteFile` / `WebsiteManifest` and `SourceKind` `"website"`), DB schema,
project format, `design.md` / tokens, OpenAI / Anthropic wire (one extra tool
definition only). No migration. v0.0.0–v0.2.0 projects and settings open as-is.
