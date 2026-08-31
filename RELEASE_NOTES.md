# WAKARU v0.2.0

A full re-development of WAKARU on Tauri v2 (Rust backend + a from-scratch
React frontend built to the Hallmark design system). v0.1.0's application source
was never published and could not be recovered; v0.2.0 is a clean
re-implementation from the product specification and the v0.1.0 IPC contract.

macOS (Apple Silicon) only for this release, ad-hoc signed and **not
notarized** — on first launch, right-click the app and choose *Open* to get
past Gatekeeper.

## What's in it

- **Projects & ingestion** — self-contained project folders; add PDFs, images,
  audio, video, spreadsheets, text/Markdown/JSON/code, and web links (with an
  SSRF guard). PDF text, Office formats, and CJK bi-gram full-text search.
- **Viewer** — tabbed panes, previews for every format, in-preview find,
  reader view for web links.
- **RAG + AI** — one OpenAI-compatible client (no vendor SDK); connection
  profiles with keys in the macOS keychain; hybrid FTS + vector search (RRF)
  with a keyword-only fallback when offline.
- **Live Illustrator** — on-demand, cached page explanations with citations
  resolved on the Rust side (the model never invents page numbers).
- **Audio / video** — local transcription with whisper.cpp (Metal). Download a
  model in Settings; transcripts are time-stamped and clickable to seek.
- **Studio** — free chat tabs over a project's sources, with built-in tools
  (`search_sources`, `read_document`, `read_file`, `write_file`, `run_command`,
  …), `@`-mention of other tabs, a `workspace/` folder, and artifact cards you
  can add back as sources or download.
- **MCP** — connect local stdio MCP servers; per-tool approval policy
  (ask / always allow / deny), revocable from Settings.
- **Sandbox** — everything `write_file` / `run_command` touch is confined to
  `workspace/`: no shell, scrubbed environment, path-traversal and symlink
  escapes rejected, output capped, timeouts kill the whole process group.
- **File Modifier** — images → PDF, and paste → organised Markdown/TXT.
- **Settings & i18n** — English / 日本語 / 简体中文 on every screen; light /
  dark / system themes; monochrome mode; six display sizes; first-run wizard.
- **Export / import / archive** — portable `.wakaru.zip` with a schema version
  and a four-case compatibility policy.

## Known limitations

Documented in `docs/DECISIONS.md`:

- **D-09** PDF / slide page-image rendering — the Viewer shows extracted text.
- **D-10** Local embeddings — remote `/v1/embeddings` only; FTS-only degrade
  when no search model is set.
- **D-11** Ingest-time Vision analysis of pages/images.
- **D-13** Studio chat is request/response, not token-streamed.
- **D-14** MCP — stdio transport only; Streamable HTTP is deferred.
- **D-15** Transcription uses an energy-gate VAD (not Silero); video keyframe
  extraction is not included (audio is still transcribed; the player handles
  video). Whisper model files are integrity-checked by a recorded SHA-256
  rather than an upstream-pinned hash.
- Windows / Linux builds are unverified and not distributed.

## Verification

`docs/QUALITY_REPORT.md` (this repo) records the gate results for the build.
