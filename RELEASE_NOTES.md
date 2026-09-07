# WAKARU v0.1.0

WAKARU v0.1.0 makes local and LAN AI workflows more reliable without changing
the application hierarchy, project format, or established workflows.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Highlights

- **Reliable local streaming** — event listeners are ready before a request starts,
  preventing an early streamed response or error from being lost on fast local
  models.
- **Accurate tool detection** — the connection check gives compatible local models
  enough bounded output space to emit a tool call, avoiding false “tools not
  supported” results.
- **Clearer AI failures** — Live Illustrator and text conversion now present
  localized, actionable failures instead of raw technical messages.
- **Japanese and English coverage** — Live Illustrator grounding and UI error
  states were exercised in both languages.
- **Safe compatibility** — project data, profile storage, sandbox boundaries,
  cancellation, and existing screen structure remain unchanged.

## Compatibility and limits

- Existing v0.0.0–v0.0.6 projects and settings open unchanged. There is no database
  migration in this release.
- OCR, document generation, Viewer rendering, search, local/LAN model connections,
  MCP, and all other existing capabilities remain available. When an endpoint does
  not implement embeddings, search safely continues with its built-in text index.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified in
  this release.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
