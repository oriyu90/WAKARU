# WAKARU v1.0.0

WAKARU v1.0.0 is the first formal stable release. It consolidates the
document-centred Viewer, source-grounded Live Illustrator, project RAG and the
Studio workspace while hardening the failure paths found in the release audit.

## What changed

- Studio document generation now makes the selected format authoritative. PDF
  and Word output always receives the matching filename and MIME type, so files
  import back into View materials through the correct renderer.
- Studio writes artifacts through a flushed same-directory temporary file and
  atomically replaces the destination, preventing half-written PDF/DOCX files
  after a crash or disk error.
- OpenAI-compatible connections now distinguish malformed HTTP 200 responses,
  invalid SSE and completed-but-empty model turns from ordinary truncation.
  Valid LM Studio and MLXBar streams without a `[DONE]` marker remain supported.
- Imported or Studio-authored website previews retain JavaScript but now run in
  an opaque-origin iframe without same-origin or form permissions.
- PDF page controls no longer collapse into vertical text beside Live
  Illustrator. Interactive PDF, Word and PowerPoint rendering is capped at
  96 MiB to avoid multi-copy WebView memory spikes; larger sources still ingest,
  search and expose extracted text.
- The official four-language product page has been rebuilt with a calm Hallmark
  workbench presentation, clearer Viewer → Live → Studio flow and explicit
  local-first / distribution boundaries.

## Compatibility

- Existing projects, settings, Studio conversations and Live Illustrator
  sessions from v0.0.0 through v0.3.0 remain compatible.
- No database migration, project archive change or AI profile migration is
  required.
- Live Illustrator remains separate from Studio until the reader explicitly
  chooses the handoff action, and a source-scoped explanation remains one
  session while its pages change.

## Distribution

Apple Silicon Mac, macOS 12 or later. The application is MIT licensed. This
build is ad-hoc signed and is not Apple-notarised. Distribution remains
invite-only while the GitHub repository is private.
