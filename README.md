# WAKARU — source

Local-first AI notebook for your own documents. Tauri v2 + Rust backend + React 19 / TS
frontend.

> This is the **development** repository. It is not pushed to `oriyu90/wakaru` — that
> public repo carries only `README` / `LICENSE` / release notes / website (project
> policy). See `common-rules-document/WAKARU.md`.

## Status

Re-development toward **v0.2.0** (a from-scratch Hallmark UI over a rebuilt backend —
the v0.1.0 source was lost). Work follows `docs/08_実装フェーズ計画.md` P0 → P11.

- **P0 (scaffold + design system)** — done: Tauri v2 shell, `app.db` + migrations,
  job registry skeleton, `AppError` + `ts-rs` bindings, the locked design system
  (`design.md` / `src/styles/tokens.css`), all 8-state base components + the
  `__preview__/states` demo, `AppShell` with the translucent overlay sidebar.

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
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

Commits are prefixed `phase(N):`.

## License

MIT © 2026 Yuki Orita
