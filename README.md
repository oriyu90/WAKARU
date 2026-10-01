# WAKARU

**Local-first AI notebook for your own documents.** Tauri v2 + Rust backend, React 19 /
TypeScript frontend. Author: **Yuki_Orita**. License: **MIT**.

WAKARU imports your sources, renders them properly (PDF, DOCX, PPTX, spreadsheets,
Markdown, images, audio, video, web), and lets you ask questions grounded in them —
through any OpenAI-compatible or Anthropic-compatible endpoint, including local model
servers on your machine or LAN. Search, Live Illustrator explanations and the Studio
agent all run against your own material; file viewing and full-text search work fully
offline.

## Download

Prebuilt macOS (Apple Silicon) builds are on the
[Releases page](https://github.com/oriyu90/WAKARU/releases/latest) (this
repository is currently private, so release assets are available to
collaborators). Verify the `.dmg` against its `.sha256`, open it, and drag
**WAKARU** to Applications. The build is ad-hoc signed and not Apple-notarized,
so on first launch right-click WAKARU → **Open**.

## Highlights

- **Bring your own model** — per-connection OpenAI-compatible (`/chat/completions`,
  Bearer) or Anthropic-compatible (`/messages`, `x-api-key`) wire format. Presets cover
  OpenAI, Gemini, OpenRouter, Claude, GLM/Z.AI, LM Studio, Ollama and MLXBar, while
  custom compatible servers can run locally or on your LAN. Reasoning
  models are handled either way — a `reasoning_content` field or an inline
  `<think>…</think>` block is kept out of the answer. Connection checks verify
  actual generation with the selected model and identify stopped MLXBar models.
- **Real rendering** — PDF via bundled PDF.js, DOCX/PPTX structure, bounded sheet
  tables, Markdown, images, audio/video with transcripts. Studio-generated Markdown,
  PDF and other artifacts can be imported straight into Viewer, and queued imports
  refresh into their final preview without reopening the project. PDF, DOCX and PPTX
  views include non-destructive invert, contrast and bounded PDF sharpening controls.
  Document zoom and a full-width reading mode make dense material easier to read;
  closing Live Illustrator returns its width to the document.
- **OCR** — scanned PDFs and images with text are recognised (pure-Rust `ocrs`,
  models fetched once) and folded into search; a scanned PDF also gets a
  `searchable.pdf` with an invisible, position-matched text layer.
- **Studio creation recipes** — even a small local model gets explicit source-reading,
  web-search, command and document-building recipes. `build_document` deterministically
  assembles Markdown, Word or PDF; File Modifier can also save organised text in all
  four formats.
- **Hybrid search** — FTS + optional vector search, with a locally cached
  multilingual embedding model as an offline fallback.
- **Live Illustrator** — plain-language, understanding-oriented explanations with
  citations. Follow-up questions use bounded conversation-aware RAG in the selected
  page/source/project scope. Source-level sessions stay intact while pages change,
  and enter Studio only after the explicit **Hand off to Studio** action.
  Page-scoped questions include visible-page evidence even when search wording
  differs from the source.
- **Studio** — an agentic loop over your sources and a sandboxed workspace, with tool
  approval, iteration limits and artifact export. Drop files from Finder to copy and
  ingest them into the project; completed tool work stays in expandable activity rows.
  Enter sends, Shift+Enter inserts a newline, and an earlier user turn can be edited.
  Follow-up searches respect the selected source and only cite retrieved chunks.
  Sent text appears immediately with persistence reconciliation, invalid tool calls
  stop after a repeat with guidance, and a whole source can be translated into a
  verified PDF via `translate_source_document`. `create_visual_preview` builds small
  interactive figures shown inline in the conversation.
- **Sticky notes** — right-click the material to pin a private memo. Cards stack in
  reading order, survive zoom, window resizes, restarts and ZIP round-trips, and are
  never sent to AI search or prompts.
- **Large sources** — multi-hundred-megabyte text, sheets and PDFs open in bounded
  windows/pages (byte ranges, absolute line/row numbers) instead of loading the
  whole file into the view; over-limit indexes report partial coverage honestly.
- **MCP** — local stdio servers (a bare `npx` / `uvx` / `node` command is found
  in the usual install locations; a SearXNG web-search preset is built in) and
  policy-checked Streamable HTTP servers. Connected SearXNG/Tavily search tools are
  exposed to Studio and explicit Live Illustrator web requests through one stable tool.
- **Private by default** — API keys in the OS keychain; original files served only
  through a project-scoped `wakaru-asset://` protocol; no telemetry.
- Japanese / English / Simplified Chinese UI, light / dark / monochrome, WCAG AA.
- A neutral, conversation-first interface with the existing WAKARU
  navigation and document-work hierarchy preserved.

## Build from source

```bash
npm install
npm run tauri dev          # develop
npm run tauri build        # package (macOS: add -- --bundles app for an app-only build)
```

Requirements: Node 20+, a stable Rust toolchain (see `src-tauri/Cargo.toml`
`rust-version`), CMake (for the bundled `whisper.cpp`), and Xcode command-line
tools on macOS.

## Repository layout

| path | what |
|---|---|
| `design.md` | the locked design system — read before touching any `*.module.css` |
| `docs/` | product spec (`00`–`09`), `DECISIONS.md`, `GLOSSARY.md`, `ipc-contract.md`, `HANDOFF.md` |
| `src/` | frontend — `app/` shell + routing, `components/` primitives, `features/` screens, `ipc/` wrappers, `i18n/`, `stores/` |
| `src-tauri/src/` | backend — `commands/` (thin), `domain/` (ts-rs types), `services/`, `storage/`, `jobs/` |
| `src-tauri/migrations/` | `app/` and `project/` SQL migrations |
| `scripts/` | `check-design-rules` · `check-contrast` · `check-i18n` · `check-hardcoded` · `gen-licenses` |

## Verification gate

```bash
npm run typecheck && npm run lint && npm test && npm run check:contrast && npm run check:i18n && npm run build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo deny check
```

## License

MIT © 2026 Yuki_Orita. See [LICENSE](LICENSE). Third-party components:
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
