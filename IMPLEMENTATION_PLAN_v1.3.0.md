# WAKARU v1.3.0 implementation plan

## Findings

1. The Live Illustrator drawer keeps its persisted inline width while closed.
   That inline style wins over the zero-width closed rule and permanently steals
   space from the reader.
2. PDF/PPTX page changes are persisted to SQLite, but the React Query tab cache
   keeps the old locator. Switching away and back therefore recreates the preview
   from stale data.
3. Studio already has source search/read, deterministic document builders, a
   sandboxed command runner and MCP tools. The missing layer is a compact,
   low-capability-model recipe that explains when to use each tool. Web-search MCP
   tools are exposed under generated names, which is especially hard for small
   models, and Live Illustrator has no MCP path.
4. Studio has no native file-drop import. Its workspace column only displays files
   created by tools.
5. The protocol adapters cover OpenAI Chat Completions and Anthropic Messages,
   including streaming and tool calls, but Settings lacks first-class presets and
   contract coverage for Gemini, OpenRouter and GLM endpoints.
6. File Modifier saves organised text only as Markdown/TXT even though the app has
   deterministic DOCX/PDF builders.

## Implementation

- Make the drawer truly zero-width and non-interactive while closed; retain the
  persisted width only for its open state.
- Optimistically update the viewer-tab locator cache whenever a page/slide changes,
  while continuing to persist the same locator in SQLite.
- Add a stable built-in `web_search` Studio tool that routes to a connected
  SearXNG/Tavily/search MCP tool. Add explicit, short source/document/web/command
  recipes and an actual command catalogue to the Studio system prompt.
- Let Live Illustrator use the same MCP search bridge when the reader explicitly
  requests a web/current-information lookup; keep results marked as untrusted data.
- Add native Tauri drag/drop handling in Studio. Copy dropped files atomically into
  `workspace/imports`, register them as artifacts in the right column, and ingest
  them as project sources so the AI can search/read their extracted content.
- Group assistant tool requests and tool results into collapsed activity blocks,
  while keeping approval controls visible when action is required.
- Add official presets for Gemini, OpenRouter, GLM/Z.AI, OpenAI, Anthropic, Ollama,
  LM Studio and MLXBar; add endpoint-normalisation and wire-format tests.
- Extend File Modifier output to deterministic DOCX/PDF in addition to Markdown/TXT,
  using atomic writes and matching extensions.
- Update Japanese, English and Simplified Chinese UI strings and the design/spec,
  decision, handoff, README, release and quality documents.

## Verification and release

- Frontend: typecheck, lint/design/hardcoded checks, unit tests, contrast, i18n,
  production build, and rendered narrow/normal Studio/Viewer checks.
- Backend: format, clippy with warnings denied, all non-manual tests, cargo-deny.
- Real endpoint: model listing, streaming text, tool call and document/tool workflow
  against the owner-provided MLXBar endpoint without persisting its key.
- Release: bump all five version locations, build the arm64 app with ad-hoc signing,
  create/verify/mount the DMG, verify strict codesign and launch, scan for secrets,
  commit/push/tag, create the GitHub Release, then update and validate Studio RIZI.
