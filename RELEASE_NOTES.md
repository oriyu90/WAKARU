# WAKARU v0.2.2

WAKARU v0.2.2 makes Studio artifacts and presentation files dependable in
Viewer, makes the Live Illustrator → Studio boundary explicit, and hardens
AI-context and project-file handling. There is no database or project-format
migration.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Viewer and Studio

- Studio-generated Markdown, PDF and other document artifacts now import as
  project sources and open directly in Viewer. Generated website folders from
  v0.2.1 remain supported.
- Viewer refreshes a queued source's detail, document and asset caches when
  ingestion completes, so the final preview appears without reopening it.
- PPTX previews now render even when the parser's slide count already matches
  the source metadata. Binary preview queries also serialize file sizes safely.

## Live Illustrator

- A whole-source Live Illustrator session remains the same session when you
  move between pages of that source. Its automatic explanation is generated
  once from bounded whole-source context.
- Live sessions are not added to Studio automatically. The explicit **Hand off
  to Studio** action is awaited, reports success or failure, and transfers the
  saved explanation plus the conversation atomically.

## Context and project safety

- Untrusted source and mention context is UTF-8-safely bounded before an AI
  request is assembled.
- Project ZIP import rejects traversal paths, caps entry count and expanded
  size, and cleans up incomplete imports on failure.
- Artifact paths and project/source identifiers are revalidated at filesystem
  boundaries. Re-ingestion updates the global search index by project and
  source, avoiding collisions between projects.

## Compatibility and limits

- Existing v0.0.0–v0.2.1 projects and settings open unchanged. There is no
  database migration in this release.
- No dependency was added or removed. The v0.2.1 website-source, document
  navigation, Studio `build_site`, OCR, search, local/LAN model, and MCP
  workflows remain available.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or
  verified in this release.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
