# WAKARU v0.0.4

WAKARU v0.0.4 adds text recognition for scanned documents and a document builder
for the Studio agent. Existing projects and settings open unchanged — no database
migration is required beyond one additive column.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Highlights

- **OCR for scanned PDFs** — when an imported PDF has pages with no text layer,
  the Viewer shows a **Recognise text** button. It rasterises each scanned page,
  runs a pure-Rust OCR engine (`ocrs`, models downloaded once on first use), and
  folds the recognised text into full-text and semantic search. A
  `searchable.pdf` — the page image with an invisible, position-matched text
  layer — is written alongside the source. Pages that already have a text layer
  are left untouched.
- **OCR for images** — an imported image that contains text is recognised at
  ingest time. The text becomes searchable and is also written next to the image
  as `ocr.txt`. A confidence heuristic keeps noise and patterns from producing
  spurious "text".
- **`build_document` in Studio** — a new built-in tool that assembles a
  formatted **Markdown, Word (.docx) or PDF** document from a title and a list of
  sections (each a heading and a Markdown body). The model supplies structure,
  not layout, so even a small local model produces a well-formed document, and
  the request stays compact. PDF output uses a Simplified-Chinese/Japanese-capable
  system font; where none is available the Markdown version is written instead.
- **Text-recognition setting** — Settings › General has an **OCR** switch
  (on by default) that governs both image OCR and the scanned-PDF prompt.

## Compatibility and limits

- macOS 12 or later on Apple Silicon. Windows and Linux are not built or
  verified in this release.
- Ad-hoc signed, not Apple-notarized.
- v0.0.0–v0.0.3 projects open unchanged. `project.db` gains one nullable
  `sources.ocr_status` column; an older WAKARU can still open a v0.0.4 project
  and preserves the column.
- The OCR models (~14 MB total) download from the network on first use; OCR
  degrades cleanly to "unavailable" when offline.
- Generated PDFs that contain CJK text embed a subset of a system font and are
  correspondingly larger than Latin-only PDFs.
- All feature limits documented for v0.0.2/v0.0.3 still apply.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
