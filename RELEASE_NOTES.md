# WAKARU v0.1.2

WAKARU v0.1.2 is a maintenance release. It fixes a Settings bug where toggle
switches did not respond to a mouse click or tap, and it makes stdio MCP servers
easier to run — a bare `npx` / `uvx` / `node` command is now found automatically,
and there is a one-click SearXNG web-search preset. The application hierarchy,
project format, IPC contract, database schema, and every existing workflow are
unchanged. There is no database migration.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**, then approve WAKARU under
System Settings → Privacy & Security → Local Network if you connect to a model
server on your network.

## Fixes

- **Settings switches respond to click and tap again** — every on/off switch in
  Settings (Enable Live Illustrator, Check for updates on startup, OCR, Prefetch
  next page, Carry over the previous page's conversation, and the rest) could
  only be toggled with the keyboard: the switch's decorative track sat on top of
  the actual control and absorbed the pointer. Clicking or tapping a switch now
  toggles it, in every language and both themes. Keyboard operation and the focus
  ring are unchanged.

## MCP

- **`npx` / `uvx` / `node` stdio servers start without an absolute path** — a
  macOS app launched from Finder inherits only a minimal `PATH`, so a stdio MCP
  server registered as `npx …` previously failed to start unless you hunted down
  the full path. WAKARU now also looks in the usual install locations (Homebrew,
  `~/.local/bin`, Cargo/Bun/Deno/Volta, nvm/fnm) and passes that same widened
  `PATH` to the server process. Servers registered with an absolute path are
  unaffected. The command is still started **without a shell**, and the
  secret-free environment allowlist is unchanged.
- **SearXNG web-search preset** — the "Add MCP server" dialog has a preset
  chooser. Picking **SearXNG (web search)** fills in a known-good stdio
  configuration (`npx -y mcp-searxng`, `SEARXNG_URL=…`). You still review every
  field and save it yourself; nothing connects on its own. You need your own
  running SearXNG instance with its JSON output format enabled.

## Compatibility and limits

- Existing v0.0.0–v0.1.1 projects and settings open unchanged. There is no
  database migration in this release.
- OCR, document generation, Viewer rendering, search, local/LAN model
  connections, MCP, and every other existing capability remain available. When an
  endpoint does not implement embeddings, search safely continues with its
  built-in text index.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified
  in this release.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
