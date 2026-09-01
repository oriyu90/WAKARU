# WAKARU v0.0.1

A reliability and responsive-layout update to the fully redeveloped WAKARU app
on Tauri v2, Rust, and React. It supersedes the v0.0.0 pre-release while
preserving existing projects and settings.

macOS (Apple Silicon) only for this release, ad-hoc signed and **not
notarized** — on first launch, right-click the app and choose *Open* to get
past Gatekeeper.

## What's in it

The v0.0.0 line repaired
legacy-database imports, production Markdown/CSV previews, the collapsed search
field, keyword-only embedding fallback, and newly disclosed PDF/XML denial-of-
service risks. v0.0.1 adds the responsive, compatible-API, and MCP hardening
described below. Existing project data is migrated in place.

This release also hardens document work for smaller models. Source
text, retrieved excerpts, file contents, and tab transcripts are now separated
from system instructions and explicitly treated as untrusted data. Studio uses
a short verify-before-writing workflow and must create a workspace file when a
document is requested. The text organiser blocks Markdown saving when its output
falls below 70% of the source, and primary dark-theme text is now exact white.

- **Two API formats** — choose OpenAI-compatible or Anthropic-compatible per
  connection. The Anthropic adapter supports native authentication, top-level
  system instructions, image blocks, tool definitions/results, structured
  output mapping, text/thinking/tool SSE events, usage, and provider errors.
- **Built-in response quality** — Studio continuously avoids canned AI prose,
  filler, repetition, and unnecessary structure. Live Illustrator combines a
  plain-language first explanation with examples, misconception handling, and
  an optional short understanding check. No external Skill source is bundled.
- **Safer document workflows** — page text and RAG excerpts are passed as
  bounded user data rather than system instructions; Studio file requests use
  the complete `write_file` path, and suspiciously shortened organiser output
  cannot be saved as Markdown.
- **Reliability hardening** — validated endpoint URLs, safe database migration
  for existing profiles, incomplete-stream and output-limit detection, no
  caching of truncated explanations, recoverable poisoned locks, safe corrupt-ZIP errors, and
  visible failures when saving AI settings.

- **Projects & ingestion** — self-contained project folders; add PDFs, images,
  audio, video, spreadsheets, text/Markdown/JSON/code, and web links (with an
  SSRF guard). PDF text, Office formats, and CJK bi-gram full-text search.
- **Viewer** — tabbed panes, previews for every format, in-preview find,
  reader view for web links.
- **RAG + AI** — one protocol-neutral client (no vendor SDK); connection
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
  (ask / always allow / deny), revocable from Settings. The client prefers MCP
  2026-07-28 discovery and falls back to legacy initialization, bounds every
  network/process wait, refreshes tool catalogs, and preserves standard tool
  result content without expanding binary base64 into the model context.
- **Responsive workbench** — settings, Studio, Viewer drawers, source rows and
  file tools adapt down to narrow portrait and low landscape windows instead
  of losing the right or bottom half of the interface.
- **Safer compatible-API setup** — LAN HTTP endpoints remain supported; base
  URLs, custom headers and timeouts are validated, while capability tests now
  verify real tool calls and valid structured JSON instead of transport success alone.
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

`QUALITY_REPORT.md` records the gate results for the build.
