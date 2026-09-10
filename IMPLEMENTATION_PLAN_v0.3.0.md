# WAKARU v0.3.0 implementation plan

Date: 2026-09-10

## Findings

1. The native PDF, DOCX and PPTX renderers have navigation and zoom controls,
   but no document-local visual adjustment state. A CSS-only “sharpness” label
   would be misleading because CSS has no sharpening primitive.
2. Live Illustrator retrieval embeds/searches only the newest question. It does
   not send prior Q&A to the model, so follow-ups such as “why is that?” lose
   their referent. Its `page` scope currently applies only a source filter and
   is therefore equivalent to `source` scope.
3. Studio currently sends only with Cmd/Ctrl+Enter. The requested Enter and
   Shift+Enter behavior is absent.
4. Studio messages are immutable from the UI and IPC. A safe rewind must not
   destroy later turns merely by clicking an edit affordance.
5. The generic OpenAI-compatible client already handles MLXBar's
   `/v1/models`, Chat Completions SSE, reasoning deltas, tool deltas, usage and
   `[DONE]`. MLXBar has no first-class preset, and the compatibility contract is
   not protected by an MLXBar-shaped regression test.

## Decisions

### D-36: Per-view document enhancement, no source mutation

- Add a compact viewer toolbar for PDF, DOCX and PPTX with an invert toggle and
  one combined clarity control.
- The control raises contrast for every renderer. PDF additionally receives a
  bounded 3×3 unsharp pass on the current canvas page; the pass is skipped over
  a pixel ceiling to prevent a second large image buffer from causing memory
  pressure. DOCX text and PPTX SVG remain vector-sharp and therefore do not need
  bitmap convolution.
- Adjustments are presentation-only React state. They never alter imported
  bytes, OCR input, extracted text, citations, exports or project files.

### D-37: Conversational, scope-correct Live RAG

- Pass the viewer's current locator with each question while preserving one
  Live thread per source.
- Search `page` within the current document ordinal, `source` within the current
  source and `project` across the project.
- Expand retrieval queries with the most recent user question so pronoun-heavy
  follow-ups retain a subject. Use bounded OR keyword matching (plus the
  existing embedding/RRF path) for recall, and keep the request/response history
  under a fixed byte budget.
- Send bounded prior user/assistant turns before the current RAG question. The
  source excerpts remain explicitly untrusted and current-turn citation tags
  remain server-resolved.

### D-38: Deferred, transactional Studio branching

- Every completed user message can enter “edit from here” mode. Clicking only
  copies the old text into the composer; it does not mutate the database.
- Submitting in edit mode validates that the target belongs to the active Studio
  thread, transactionally removes that user message and all later messages,
  inserts the edited turn, then runs the normal model loop.
- Cancel exits edit mode with history intact. Artifacts remain project-owned as
  before. Pending approval/continuation and active streams cannot be rewound.
- A shortcut beside the composer selects the most recent user message, covering
  the common “edit previous message” case.

### D-39: MLXBar as an explicit OpenAI-compatible target

- Add an `MLXBar` preset at `http://127.0.0.1:11435/v1` and localized setup
  guidance, while retaining custom URLs and bearer-token entry for LAN mode.
- Do not couple WAKARU to MLXBar's private management API or knowledge-base DB;
  WAKARU remains the owner of project RAG and sends ordinary OpenAI-compatible
  Chat Completions. This keeps LM Studio, Ollama and cloud providers compatible.
- Add an MLXBar-shaped SSE/model-list contract test, including keep-alives,
  `reasoning_content`, terminal reason, usage and `[DONE]`.

## Work plan

1. Implement shared document controls and bounded PDF sharpening; add UI and
   pure-function tests, responsive styling and ja/en/zh-Hans strings.
2. Correct Live RAG scope/query/history construction and add Rust/integration
   regression coverage for follow-up retrieval and page isolation.
3. Add Studio Enter/Shift+Enter behavior and deferred transactional rewind/edit
   APIs, UI states and tests.
4. Add the MLXBar preset/guidance and wire-contract regression coverage.
5. Run bindings, typecheck, lint/design/hardcoded, unit/integration tests,
   contrast/i18n, production build, Rust fmt/clippy/test and dependency audit.
6. Inspect real native screens in light/dark and normal/narrow states. Update
   decisions, README, release notes/draft, quality report and handoff.
7. Set all package versions to 0.3.0, build and ad-hoc sign the arm64 app/DMG,
   verify signatures/checksum/startup, publish the GitHub release, update the
   four-language Studio RIZI pages, and append private maintenance notes.

## Compatibility and rollback

- Additive IPC fields are optional/defaulted; no project DB schema or project
  archive format bump is required.
- Existing v0.0.0–v0.2.2 projects, profiles and role bindings remain readable.
- Viewer adjustments reset with the renderer and cannot corrupt source data.
- Revert D-36 independently if WebKit filter rendering regresses; RAG, Studio
  history and provider behavior do not depend on it.
