# WAKARU v1.6.0 design record

Date: 2026-10-01. Scope: `IMPLEMENTATION_PLAN_v1.6.0.md` — large-source intake
and reading, interactive AI figures, source-bound sticky notes.

## Positions

- **Large sources are staged, never swallowed.** The original is always copied
  verbatim; the searchable index is a bounded head window and anything beyond
  is `ready_partial` with the unparsed range — never "fully indexed". The
  WebView receives 64–256 KiB text windows, single Range responses (32 MiB
  cap, 512 MiB unbounded-GET refusal), and PDF pages via
  `rangeChunkSize: 256 KiB` with a bounded-buffer fallback. The 96 MiB
  interactive ceiling (Office) and the 16M-pixel canvas ceiling stay.
- **Figures are typed artifacts, not markup in chat.** `VisualPreview`
  (`html+css+js` ≤ 256 KiB, state ≤ 16 KiB, no remote/CDN/import/fetch/forms)
  is stored in `visual_previews`, linked from `message_visuals` and
  `illustrations.visual_id`. Studio builds them with `create_visual_preview`;
  Live with `illustrator_generate_visual`. Both render through one
  `InteractivePreview`: opaque-origin `sandbox="allow-scripts"`, meta CSP
  (`connect-src 'none'`, no frames/forms), no IPC bridge, token-checked
  state notes only.
- **Notes are private memos on the material.** `notes` rows carry
  `(source_id, locator, anchor_kind, anchor_json)` with finite `x/y ∈ [0,1]`
  page anchors or section/offset+fingerprint text anchors, 4,000-char plain
  text, and soft delete + restore. Right-click creates, marker right-click
  deletes with undo, Shift+F10 creates at the current position. Positions are
  surface fractions, so zoom/DPR/resize re-project for free. Notes never
  enter AI search or prompts.
- **Archives stay honest.** `004_notes_visuals.sql` bumps
  `PROJECT_SCHEMA_VERSION` to `1.1.0` (forward-only; old DBs migrate, old
  ZIPs import as zero notes/visuals). Export streams entries (`io::copy`),
  snapshots the DB with `VACUUM INTO`, writes `.part` + rename, and records
  note/visual counts + digest. Import checks expansion/files/paths/symlinks,
  compression ratio, `integrity_check`, and note anchor references.

## Explicit non-goals

- Full-streaming ingest (copy+hash background jobs with progress/cancel UI)
  is not in this cut: intake stays synchronous with a new free-space guard
  (`SOURCE_NO_SPACE` with needed/missing bytes), iCloud/permission
  distinctions, and index ceilings.
- 3D figures and arbitrary npm execution stay out; v1.6.0 figures are 2D
  SVG/Canvas + bounded inline script, math as generated SVG.
- Pixel-anchored web-page notes stay out: external/live sites take reader-text
  anchors only.
