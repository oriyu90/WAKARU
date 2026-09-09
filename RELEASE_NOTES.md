# WAKARU v0.2.1

A follow-up to v0.2.0. It simplifies the Live Illustrator panel, adds
page/scroll navigation to the document viewer, lets you collapse tool output in
Studio, lets Studio build a website (PDF and Word were already there), and adds
importing a **website folder** into 資料を見る with a live preview of its
`index.html`. The project format, database schema, and existing workflows are
unchanged; projects from v0.0.0–v0.2.0 open as-is with no migration.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Live Illustrator

- **The two segmented controls are gone.** The panel always explains the whole
  document, at the detail level set in Settings.
- **Change the detail level from under the explanation.** Right after the first
  automatic explanation — and only while you have not asked a follow-up yet —
  two buttons offer the other levels (e.g. if Settings is 標準, you get
  かんたん and くわしい). Tapping one regenerates at that level and replaces the
  explanation in place.

## 資料を見る — navigation

- **Arrow keys turn PDF and slide pages.** `←` / `→` and `PageUp` / `PageDown`
  move between pages in the PDF and PowerPoint renderers (they only responded to
  the toolbar buttons before). Keys are ignored while you are typing in a field.
- **Hover arrows on the page.** Large `‹` / `›` controls appear on the left and
  right of a PDF or slide when there is more than one page.
- **Scroll buttons for long documents.** Word documents and the Markdown / web
  reader get an up/down control (and `PageUp` / `PageDown`) that scrolls the view
  by about one screen.

## Studio

- **Collapse tool output.** A tool result is now a collapsed block you expand
  when you want to read it, the way Claude shows them. The approval card still
  shows the full arguments.
- **Build a website.** A new `build_site` tool writes a small static site
  (HTML/CSS/JS/assets) as one folder in `workspace/`. "Add as source" imports it
  as a website source you can preview and cite; "Download" saves the folder.
  Building formatted PDF and Word documents (`build_document`) is unchanged.

## Website folders in 資料を見る

- **Add website** in the viewer rail opens a folder picker. WAKARU copies the
  folder into the project (static web files only; `node_modules`, dotfiles and
  symlinks are skipped; bounded to 4000 files / 128 MB), recognises the entry
  page (`index.html`, else the shallowest `.html`), and extracts the readable
  text of every HTML file so Studio and Live Illustrator can use it.
- **Open it to preview.** A file tree on the left, and the selected page rendered
  live on the right in a sandboxed frame served from the project's own asset
  scheme — scripts run, but the frame cannot reach the app. Relative and
  root-absolute links between the site's files resolve inside the project
  sandbox.

## Compatibility and limits

- Projects and settings from v0.0.0–v0.2.0 open unchanged. **No database
  migration.** New IPC calls `source_add_folder` and `website_manifest`; the
  bindings gain `WebsiteFile` / `WebsiteManifest` and a `"website"` member on
  `SourceKind`. The only AI-wire change is one extra function tool (`build_site`)
  in the `tools` array — the OpenAI and Anthropic request/stream formats are
  otherwise unchanged.
- The website preview frame runs with `sandbox="allow-scripts allow-same-origin
  allow-forms"`. Imported sites share one local asset origin, so one imported
  site could read another's files if it knew the path; all content is local and
  user-supplied. See `docs/DECISIONS.md` D-33.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.

---

## Previously in v0.2.0

Vertical tab rail in 資料を見る; opened documents fit the window and re-fit on
resize; Live Illustrator got a visible toggle and explains the whole document on
open; local-model (LM Studio / mlx-bar / llama.cpp) replies fixed, including a
base URL that omits `/v1`; pick a model from the server's `GET /models` list;
past Studio conversations stay closed until opened; a sent Studio message clears
from the composer; fullscreen drops the empty traffic-light inset.
