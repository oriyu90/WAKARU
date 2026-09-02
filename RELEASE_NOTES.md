# WAKARU v0.0.2

WAKARU v0.0.2 completes the document-viewing path and closes the main deliberate
deferrals from v0.0.0/v0.0.1. Existing projects and settings remain compatible.

This release is for macOS Apple Silicon. It is ad-hoc signed and **not
notarized**; on first launch, right-click WAKARU and choose **Open**.

## Highlights

- **Real file rendering** — PDF pages render with bundled PDF.js; DOCX and PPTX
  render their document/slide structure; XLSX/XLS/CSV/TSV show bounded sheet
  tables; Markdown, images, audio, video, text, JSON, code and web reader views
  retain their dedicated previews. Failed rich previews fall back to searchable
  extracted text instead of leaving a blank screen.
- **Safe document boundary** — original files are served only through the
  project-scoped `wakaru-asset://` protocol. Office-rendered DOM is scrubbed of
  executable elements, event handlers and unsafe links. Interactive previews
  are size-bounded and cancellable.
- **Offline semantic search** — a locally cached multilingual-e5-small model is
  used when no remote embedding role is available. Model initialization or
  download failure degrades cleanly to full-text search.
- **Image understanding** — when a Vision role is configured, normalized image
  sources receive structured OCR, layout, diagram and uncertainty analysis. The
  result augments, rather than replaces, the original source and is searchable.
- **More responsive Studio** — text deltas and active tool state appear while a
  response is running. Existing tool approval, cancellation, context limits and
  completed-message persistence remain in place.
- **MCP over stdio or HTTP** — local child-process servers and MCP Streamable
  HTTP servers share the same discovery, timeout, approval and result-isolation
  behavior. HTTPS is accepted globally; plaintext HTTP is restricted to local
  and private-network destinations. HTTP secrets are stored in the OS keychain.
- **Stronger media processing** — Silero VAD detects speech before Whisper, with
  a deterministic energy fallback. Video previews generate bounded, clickable
  runtime thumbnails without requiring an ffmpeg sidecar.
- **Compatibility and UI** — OpenAI-compatible and Anthropic-compatible APIs,
  LAN endpoints, source-grounded teaching prompts, exact-white primary text in
  dark mode, and layouts for narrow portrait and low landscape windows remain
  supported.
- **Dependency refresh** — frontend and Rust dependency audits are clean; the
  bundled third-party license inventory is regenerated for this build.

## Compatibility and limits

- macOS 12 or later on Apple Silicon; Windows/Linux builds are not distributed
  or verified in this release.
- The app is ad-hoc signed and not Apple-notarized.
- Legacy binary DOC/PPT, encrypted or corrupt documents, and browser-unsupported
  media codecs may use textual fallback or show a recoverable unsupported error.
- PDFs with a text layer are searchable and every PDF is visually rendered.
  Scanned PDFs without a text layer are not automatically OCR-indexed in v0.0.2.
- Vision and remote AI features require a compatible configured endpoint. File
  viewing, full-text search and local embedding fallback remain local-first.

See `QUALITY_REPORT.md` for the final verification record and artifact checksum.
