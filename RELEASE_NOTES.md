# WAKARU v0.1.1

WAKARU v0.1.1 is a maintenance release. It broadens reasoning-model
compatibility, keeps the dependency-audit gate green, and fixes a small Live
Illustrator error-display issue. The application hierarchy, project format, IPC
contract, database schema, and every existing workflow are unchanged.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**, then approve WAKARU under
System Settings → Privacy & Security → Local Network if you connect to a model
server on your network.

## Fixes

- **Reasoning models** — when an OpenAI-compatible endpoint streams a model's
  chain of thought inline as a leading `<think>…</think>` block (instead of the
  separate `reasoning_content` field), that span is now routed to the reasoning
  channel. It no longer appears, tags and all, inside a Live Illustrator
  explanation, a Studio answer, or an organized document. Models that do not emit
  such a block are completely unaffected.
- **Live Illustrator** — a question error no longer stays on screen after you
  change the detail level, move to another page, or regenerate the explanation.
- **Dependency audit** — `cargo deny check` passes again. `RUSTSEC-2024-0436`
  (the `paste` build-time macro, pulled in transitively by `fastembed`) is a
  maintenance-status advisory with no runtime component and no available
  replacement; it is now recorded as a reviewed, accepted exception with a
  written reason, alongside the existing ones.

## Compatibility and limits

- Existing v0.0.0–v0.1.0 projects and settings open unchanged. There is no
  database migration in this release.
- OCR, document generation, Viewer rendering, search, local/LAN model
  connections, MCP, and every other existing capability remain available. When an
  endpoint does not implement embeddings, search safely continues with its
  built-in text index.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified
  in this release.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
