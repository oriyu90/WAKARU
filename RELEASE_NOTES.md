# WAKARU v1.5.0

WAKARU v1.5.0 makes Studio sends reliable and completes full-document translation PDFs.

## What changed

- Sent text appears immediately in its conversation with persistence reconciliation;
  pre-persist failures restore the draft, post-persist failures keep the formal turn.
- Invalid tool calls are validated before side effects, return structured errors,
  and stop after one repeat with reselect/retry guidance; cumulative round caps
  survive “continue”, and empty answers are never treated as success.
- `translate_source_document` translates every extracted page in order and renders
  one verified `%PDF-` artifact; missing pages, limits, and render failures are
  reported without claiming completion. `build_document` now requires `format`.
- Japanese, English and Simplified Chinese UI remain in parity, with live-region
  send status, per-tab working indication, and scroll-preserving history.

## Compatibility

Existing projects, settings, conversations, sessions and AI profiles remain
compatible. No database migration is required.

## Distribution

Apple Silicon Mac, macOS 12 or later. Ad-hoc signed, not Apple-notarised.
Repository remains private; first launch needs Privacy & Security approval.

# WAKARU v1.4.0

WAKARU v1.4.0 improves local AI connection checks, source-grounded answers, and
document reading space.

## What changed

- MLXBar connection checks now generate a short reply with the selected model.
  A model that appears in the list but is stopped no longer passes the test;
  the app explains that state without exposing credentials.
- Viewer can expand the document to the full working area and restore the
  previous layout. PDF, images, Word, slides, spreadsheets, Markdown and text
  previews have visible zoom controls. Closing Live Illustrator gives its width
  back to the document. PDF rendering is bounded to avoid oversized canvases.
- Studio's follow-up source searches use the selected scope and hybrid search,
  and citations resolve only to real retrieved chunks. Source-scoped tabs cannot
  read another source. The creation guide also names the existing website tool.
- Live Illustrator keeps a new question's topic separate from older turns and
  includes visible-page chunks when keyword/vector search misses their wording.
  Vector failures safely fall back to full-text search.
- Connection and Studio errors, zoom and layout controls are localized in
  Japanese, English and Simplified Chinese.

## Compatibility

Existing projects, settings, conversations, sessions and AI profiles remain
compatible. No database or IPC migration is required. OpenAI-compatible and
Anthropic-compatible connection formats are unchanged.

## Distribution

Apple Silicon Mac, macOS 12 or later. The application is MIT licensed. This
build is ad-hoc signed and is not Apple-notarised: on first launch,
right-click WAKARU → **Open** (or allow it in System Settings → Privacy &
Security).

---

# WAKARU v1.3.0

WAKARU v1.3.0 makes document work more spacious and dependable, especially
with smaller local models: closed Live Illustrator panels return their space,
Viewer tabs remember their page, and Studio can import, read, search and create
files through clearer, inspectable workflows.

## What changed

- Closing Live Illustrator now collapses it to zero width and returns the full
  area to the document. PDF and slide page changes update both durable state and
  the active tab cache, so switching away and back restores the exact page.
- Studio gives small models concise recipes for finding source evidence, reading
  documents, building DOCX/PDF/Markdown, searching the web and using approved
  commands. Source reads identify the document/page and clearly report truncation.
- A stable `web_search` tool routes to a connected SearXNG or Tavily MCP tool.
  Explicit current-information requests in Live Illustrator use the same bridge;
  all web results remain marked as untrusted and separate from project evidence.
- Finder files can be dropped onto Studio. WAKARU atomically copies them into
  `workspace/imports`, displays them in the right rail and queues supported files
  for normal source ingestion. Tool requests and results are collapsed by default
  under expandable activity summaries.
- File Modifier can save organised text as Markdown, TXT, Word or PDF. Every
  conversion uses a same-directory atomic replace so an interrupted write cannot
  expose a partial output.
- Connection presets now include OpenAI, Gemini, OpenRouter, Claude/Anthropic,
  GLM (BigModel and Z.AI), LM Studio, MLXBar, Ollama and custom compatible APIs.

## Compatibility

- Existing projects, settings, conversations and sessions from v0.0.0 through
  v1.2.0 remain compatible.
- No database migration, project archive change or saved AI-profile migration is
  required. Presets only prefill new profile settings.

## Distribution

Apple Silicon Mac, macOS 12 or later. The application is MIT licensed. This
build is ad-hoc signed and is not Apple-notarised: on first launch,
right-click WAKARU → **Open** (or allow it in System Settings → Privacy &
Security).

---

# WAKARU v1.2.0

WAKARU v1.2.0 adds JavaScript rendering to web-link previews. A saved link
can now switch between the extracted reader view and a live, scripted view
of the original page.

## What changed

- Web links gain a reader/live switch beside the "open original" action. The
  reader view stays the default so citations keep matching the stored text;
  remote scripts run only after an explicit tap, inside the same opaque-origin
  sandbox (`allow-scripts` only, no popups, no referrer) as imported-site
  previews.
- Switching documents always returns to the reader view, and the reader text
  is not fetched while the live frame is on screen.
- The app content-security policy now permits framing `https:` and `http:`
  pages for this opt-in preview.

## Compatibility

- Existing projects, settings, conversations and sessions from v0.0.0 through
  v1.1.0 remain compatible.
- No database migration, project archive change or AI profile migration is
  required.

## Distribution

Apple Silicon Mac, macOS 12 or later. The application is MIT licensed. This
build is ad-hoc signed and is not Apple-notarised: on first launch,
right-click WAKARU → Open (or allow it in System Settings → Privacy &
Security). Distribution remains invite-only while the GitHub repository is
private.

---

# WAKARU v1.1.0

WAKARU v1.1.0 reworks the Live Illustrator panel around five reader requests:
selectable answers, an honest past-vs-session split, a proper message UI, a
freely resizable panel, and citations only when they are wanted.

## What changed

- Live answers are selectable. The auto explanation, every question and every
  answer can be drag-selected, and each carries a small copy button for exact
  retrieval. Following the conversation no longer steals an active selection.
- Past conversation works as named. Turns from before the current Live session
  stay collapsed under "Past conversation"; turns asked in this session stay
  expanded under "This session", so the newest exchange is never buried in the
  past. The detail-level rewrite buttons now consider this session only.
- The Live chat is a real message thread. User turns are end-aligned bubbles
  and assistant turns stay open on the canvas with a role label, matching the
  Studio reading contract in a drawer-compact form.
- The Live panel resizes. Drag its leading edge (or use Left/Right/Home/End
  on the handle) between 24 rem and 44 rem. The current design width is the
  minimum; widening the panel narrows the document pane, and PDF pages refit
  to the available width automatically. The width persists across restarts.
- Citations are on demand. Everyday answers carry no sources unless the reader
  asks for them; when an answer relies on excerpts, or the reader explicitly
  requests sources, `[S1]`-style citations with jump buttons appear as before.
  Small-talk turns skip retrieval entirely, and Live retrieval now uses six
  excerpts instead of twelve to suit small local models.
- Dependency maintenance: `rustls` 0.23.43 to 0.23.45 (RUSTSEC-2026-0285).

## Compatibility

- Existing projects, settings, Studio conversations and Live Illustrator
  sessions from v0.0.0 through v1.0.0 remain compatible.
- No database migration, project archive change or AI profile migration is
  required.
- Live Illustrator remains separate from Studio until the reader explicitly
  chooses the handoff action, and a source-scoped explanation remains one
  session while its pages change.

## Distribution

Apple Silicon Mac, macOS 12 or later. The application is MIT licensed. This
build is ad-hoc signed and is not Apple-notarised: on first launch,
right-click WAKARU → Open (or allow it in System Settings → Privacy &
Security). Distribution remains invite-only while the GitHub repository is
private.

---

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
