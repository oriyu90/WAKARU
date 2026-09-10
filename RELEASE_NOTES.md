# WAKARU v0.3.0

WAKARU v0.3.0 improves document readability, makes Live Illustrator follow-up
questions reliably grounded, adds editable Studio conversation branches, and
ships first-class MLXBar compatibility. There is no database or project-format
migration.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Document viewer

- PDF, DOCX and PPTX previews now have a compact black/white invert button and a
  sharpness/contrast control. Adjustments affect only the current view; imported
  originals, OCR data, search text and citations are never changed.
- PDF sharpening uses a bounded 3×3 raster pass and skips oversized canvases,
  keeping memory and interaction latency predictable. DOCX/PPTX remain vector
  content and receive contrast/inversion without rasterization.

## Live Illustrator

- Follow-up questions retrieve source excerpts using the current scope and page
  locator. The previous user question is included in a bounded retrieval query so
  pronouns and short follow-ups retain their subject.
- A bounded recent conversation is sent with the request, while factual grounding
  remains restricted to the newly retrieved excerpts. Source-level sessions still
  remain one session across page changes and enter Studio only after explicit handoff.

## Studio

- Enter sends a message; Shift+Enter inserts a newline; IME composition is not
  intercepted.
- Every previous user message offers **Edit from here**, plus a shortcut for the
  latest user turn. Choosing edit is non-destructive; submitting atomically replaces
  that user turn and the later chat tail. Existing workspace files and artifacts are
  retained.

## MLXBar

- Settings includes an MLXBar preset for `http://127.0.0.1:11435/v1`. Local/LAN
  endpoints and optional Bearer authentication use the normal OpenAI-compatible API.
- Contract tests cover MLXBar model descriptors, slash-containing model IDs, SSE
  keep-alives, reasoning deltas, usage, finish reasons and `[DONE]`. WAKARU keeps
  ownership of project RAG and does not depend on MLXBar private management APIs.

## Compatibility and limits

- Existing v0.0.0–v0.2.2 projects, settings, artifacts and Studio sessions open
  unchanged. No dependency, database schema or project archive format changed.
- Japanese, English and Simplified Chinese UI strings remain complete and in parity.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
