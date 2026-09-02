# WAKARU — source

Local-first AI notebook for your own documents. Tauri v2 + Rust backend + React 19 / TS
frontend.

> This is the **development** repository. It is not pushed to `oriyu90/wakaru` — that
> public repo carries only `README` / `LICENSE` / release notes / website (project
> policy). See `common-rules-document/WAKARU.md`.

## Status

**v0.0.2 release candidate.** The Viewer renders PDF, DOCX, PPTX, spreadsheets,
Markdown, images, audio and video rather than reducing every source to extracted
text. AI connections support OpenAI-compatible and Anthropic-compatible wire
formats; local embeddings provide an offline semantic-search fallback. Studio
streams progress and applies a concise editorial policy, while Live Illustrator
applies plain-language, understanding-oriented teaching guidance on every request.
MCP supports both local stdio and policy-checked Streamable HTTP servers.

## Layout

| path | what |
|---|---|
| `design.md` | the locked design system — read before touching any `*.module.css` |
| `docs/` | authoritative product spec (00–09), `DECISIONS.md`, `GLOSSARY.md`, `ipc-contract.md` |
| `src/` | frontend — `app/` shell + routing, `components/` primitives, `features/` screens, `ipc/` wrappers, `i18n/`, `stores/` |
| `src-tauri/src/` | backend — `commands/` (thin), `domain/` (ts-rs types), `storage/`, `jobs/` |
| `src-tauri/migrations/` | `app/` and (P1+) `project/` SQL migrations |
| `scripts/` | `check-design-rules` · `check-contrast` · `check-i18n` (run from `npm run lint` etc.) |

## Develop

```bash
npm install
npm run tauri dev
```

## Gate (every phase end)

```bash
npm run typecheck && npm run lint && npm test && npm run check:contrast && npm run check:i18n
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Commits are prefixed `phase(N):`.

## License

MIT © 2026 Yuki Orita
