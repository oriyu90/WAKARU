# WAKARU v0.1.3

WAKARU v0.1.3 is a maintenance release. It makes the Settings switches reliably
clickable in the packaged app, adds a right-click menu on the sidebar's projects
(Export / Delete), lets the Settings button toggle back to the view you came
from, fixes the "資料を見る" pane collapsing to the left half of the window, and
keeps a Live Illustrator conversation together across page turns. The application
hierarchy, project format, IPC contract, and database schema are unchanged. There
is no database migration.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**, then approve WAKARU under
System Settings → Privacy & Security → Local Network if you connect to a model
server on your network.

## Fixes

- **The "Enable Live Illustrator" switch (and every Settings switch) responds to a
  click again.** v0.1.2 fixed this for one browser engine; the packaged app uses
  another (WKWebView), where a click on the switch's decorative surface was still
  not reaching the control. The switch is now a real `<label>`, so a press
  anywhere on it toggles the control in every engine. Keyboard operation, the
  focus ring, and the accessible name are unchanged.

- **"資料を見る" fills the whole pane.** With no document open, the source list,
  its toolbar and the bottom divider were squeezed into the left part of the
  window and the right side stayed blank. The pane now uses its full width.

- **A Live Illustrator conversation stays together.** Questions you ask in the
  Illustrator drawer were filed per page, so moving to the next page appeared to
  lose the exchange. Questions and answers are now kept per document and remain
  visible as you read. Page *explanations* are still produced and cached per page.

## Changes

- **Right-click a project in the sidebar** for a small menu: **Export (ZIP)** —
  choose a folder and WAKARU writes the project archive there — and **Delete**,
  which asks for confirmation first. Both do exactly what the same actions in
  Settings → Project Management do. The context-menu key and Shift+F10 work too.

- **The Settings button is a toggle.** Press it to open Settings; press it again
  to return to the view you were on, instead of nothing happening.

- **Studio keeps the selected chat tab** after sending a message, closing a tab,
  or reloading, and a streamed reply no longer flashes twice as it is saved.

## Compatibility and limits

- Existing v0.0.0–v0.1.2 projects and settings open unchanged. There is no
  database migration in this release. No IPC, schema, or project-format change;
  the generated TypeScript bindings are byte-identical.
- Live Illustrator question threads created before this release are not deleted;
  they are simply no longer shown in the drawer, which now keeps one thread per
  document.
- OCR, document generation, Viewer rendering, search, local/LAN model
  connections, MCP, and every other existing capability remain available.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified
  in this release.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
