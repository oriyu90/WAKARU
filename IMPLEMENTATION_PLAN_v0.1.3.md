# WAKARU v0.1.3 — implementation plan

Maintenance patch. Five reader-reported problems. No IPC contract change, no DB
migration, no dependency change. Compatible with project data from v0.0.0–v0.1.2.
JA / EN / zh-Hans string parity is kept (check-i18n gate).

## Reported problems and root causes

| # | Report | Root cause | Area |
|---|--------|-----------|------|
| P1 | "ライブ解説の有効化ボタンが押せない" (still, after v0.1.2) | v0.1.2 put `pointer-events:none` on the switch's decorative spans. That fixes Chromium, but the reader runs WKWebView, where a click on an `opacity:0` `<input>` sitting *under* two `pointer-events:none` overlays is not always retargeted to the input. The control has no `<label>` association, so nothing else catches the click either. | `src/components/Switch.tsx` |
| P2 | Sidebar項目を右クリックして Export / 削除 したい | Feature not implemented — the sidebar project links have no context menu, and no menu primitive exists. | `src/app/AppShell.tsx`, new `src/components/ContextMenu.tsx` |
| P3 | 設定を開いた状態で設定ボタンをもう一度押したら元の画面に戻したい | The top-bar settings control is a plain `<NavLink to="/settings">` — pressing it while already on `/settings` is a no-op. | `src/app/AppShell.tsx` |
| P4 | 「資料を見る」で資料未オープンでも表示が左に寄り、下の境界線が右で途切れる | `Viewer` `.pane` is `display:flex` (row) but its single child (`SourceListPanel` `.panel` / `Preview` `.wrap`) has no `flex`/`width`, so it shrinks to content width and the right half of the pane stays blank. Not the Illustrator drawer. | `src/features/viewer/Viewer.module.css` |
| P5 | Studio / ライブ解説の会話が保存されない・GUIが不安定 | Backend persistence is actually correct (audited: Studio user+assistant turns → project `messages`; tabs → `studio_tabs`; Illustrator Q&A → `messages`; page explanations → `illustrations` upsert; all survive restart). The visible failure is that the **Illustrator Q&A thread is keyed per-page locator**: ask on page 3, move to page 5 → the earlier Q&A is on a different thread and looks lost. Plus two Studio GUI rough edges (active tab not restored after a refetch; streamed provisional text briefly duplicates the persisted message). | `src/features/viewer/IllustratorDrawer.tsx`, `src/features/studio/Studio.tsx` |

## Fixes

### F1 — Switch is a `<label>` (P1)
Change the `Switch` wrapper element from `<span>` to `<label>`. A `<label>` that
contains a single labelable control forwards *every* pointer press on itself to
that control, in every engine, no matter what is painted on top. Keep
`pointer-events:none` on the decorative spans as defence in depth. No API change,
no CSS change (`.switch` already has no element-specific rules). All call sites
already pass a text label via `aria-label`; none wrap `<Switch>` in an outer
`<label>`, so no nested-label hazard.
Regression test: a pointer click on the decorative track toggles the control.

### F2 — Sidebar context menu (P2)
New `src/components/ContextMenu.tsx` + `ContextMenu.module.css`:
- Rendered in a portal at the cursor, clamped to the viewport.
- `role="menu"` / `role="menuitem"`; ↑/↓ roving focus, Enter/Space activate,
  Escape and outside-click and scroll/resize close, focus returns to the trigger.
- Token-only styling; honours reduced-motion; light + dark.

Wire-up in `AppShell.tsx`: each project `NavLink` gets `onContextMenu` (and a
keyboard affordance — Shift+F10 / the context-menu key work natively on a focused
link once the handler is on it). Items:
- **Export** — `pickSaveDir()` → `exportApi.export({ ids:[id], destDir, includeEmbeddings:false })` → success/err toast (same as Project Management).
- **Delete** — confirmation `Dialog` (single project: name shown, no retype) → `projectsApi.delete(id, name)` → invalidate `["projects"]`; if the current route is `/p/:id` for the deleted project, `navigate("/", {replace:true})`.

i18n (×3): `nav.projectActions`, `nav.exportProject`, `nav.deleteProject`,
`nav.deleteProjectTitle`, `nav.deleteProjectBody`.

### F3 — Settings button toggles back (P3)
In `AppShell`, keep a ref of the last non-`/settings` pathname (default `/`).
Replace the settings `<NavLink>` with a `<button>`:
- on `/settings` → `navigate(previousPathRef.current)`
- else → `navigate("/settings")`
Preserve the active-state styling (`data-active`), and swap the `aria-label`
between `nav.settings` and `nav.settingsClose` (1 new key ×3).

### F4 — Viewer pane fills its width (P4)  ✅ already applied
`Viewer.module.css`:
```css
.pane > * { flex: 1; min-width: 0; }
```
Verified live: toolbar, divider and drop-hint now span the full pane at 1280 px
with the Illustrator disabled.

### F5 — Conversation continuity (P5)
1. **Illustrator Q&A thread is per-source, not per-page.** In `IllustratorDrawer`,
   the `thread` query key drops the locator and `getOrCreateThread` is called with
   a stable `{ t: "whole" }` locator. `illustrator_generate` keeps the exact page
   locator (page explanations stay per-page in `illustrations`). `illustrator_ask`
   already ignores the thread's stored locator for retrieval (`let _ = (thread_src,
   thread_loc)`), so this is retrieval-neutral and needs no Rust change. Old
   per-page threads are simply no longer surfaced; their rows are untouched
   (non-destructive). Default the history disclosure to open when the thread has
   messages.
2. **Studio active tab is restored.** Add an effect syncing `activeId` to
   `active.id` after a refetch so closing/creating/reloading keeps a real tab
   selected.
3. **No provisional/persisted flash.** Clear `provisional` and `runningTool` the
   moment `send` / `resolveTool` settle, before the tab refetch resolves.

Tests: Illustrator drawer keeps one thread across a page change (mock IPC);
Studio keeps a selected tab after the tab list refetches.

## Verification

Frontend: `npm run typecheck`, `npm run lint`, `npm test`, `npm run check:contrast`,
`npm run check:i18n`, `npm run build`.
Bindings: `npm run bindings` → `git diff --exit-code src/ipc/types.gen.ts` (expect
no change — no Rust type touched).
Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `cargo deny check`.
Live: browser pass over the five flows at 1280×800 and at 900×640, JA and EN,
light and dark.

## Release (v0.1.3)

1. Bump `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
   `src-tauri/Cargo.lock` (wakaru entry), `src-tauri/tauri.conf.json`.
2. `APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app`,
   ad-hoc `codesign`, `hdiutil create -format UDZO` standard DMG (app +
   `/Applications` symlink), `hdiutil verify`, mount + `codesign --verify --deep
   --strict`, startup probe for `backend ready version="0.1.3"`, `shasum -a 256`.
3. Docs: `RELEASE_NOTES.md`, `QUALITY_REPORT.md`, `docs/DECISIONS.md` (D-30),
   `docs/HANDOFF.md`, `README.md` if needed, `RELEASE_DRAFT.md`.
4. Commit (author `Yuki Orita <yukiorita0911.official@gmail.com>`), push `main`,
   tag `v0.1.3`, `gh release create v0.1.3 --latest` with the DMG + `.sha256`,
   re-download and re-verify the checksum.
5. studio-rizi: `0.1.2` → `0.1.3` across `website/projects/wakaru/**` and
   `website/content.js` (releaseVersion + a 4-language NEWS entry); `npm test` /
   `npm run build`; push `oriyu90/studio-rizi` main; confirm the Cloudflare deploy.
6. `common-rules-document/WAKARU.md`: new dated entry + refresh "現在の公開状態".
   Repository stays private; the private memo is never copied to public surfaces.

## Safety

Compatibility: no schema/'contract change; data from v0.0.0–v0.1.2 loads unchanged.
Existing features: switch a11y/keyboard, Illustrator page explanations, Studio
tools/approval all unchanged. Crash safety: context menu is portal + listeners
cleaned on unmount; no new `unwrap` on external input. Memory safety: no new Rust
`unsafe`; no Rust type/signature change. i18n: every new key added to ja/en/
zh-Hans; parity gate enforces it.
