# WAKARU v0.2.2 implementation and release plan

Date: 2026-09-10

## Objective

Make Studio-created documents reliable first-class project sources, restore
PPTX rendering in Viewer, make Live Illustrator sessions explicitly opt-in to
Studio, and keep source-scoped Live Illustrator stable while navigating pages.
Audit and harden the adjacent AI-context and project-file boundaries, then ship
the result as v0.2.2.

## Confirmed problems

1. `PptxFilePreview` stores a loaded presentation only in a ref. When the
   parser's slide count equals the ingested `pageCount` and the initial page is
   unchanged, React receives no state change and the render effect never runs.
2. Studio artifact import returns a queued source but does not take the reader
   to it. If a queued source is opened, `source://status` refreshes the source
   list only; the Viewer detail/document caches can remain stuck at the queued
   snapshot.
3. v0.2.1 correctly moved automatic Live explanations and Q&A to the canonical
   whole-source locator, but that invariant has no direct regression assertion
   covering page navigation and must remain intact through this fix.
4. The Live-to-Studio command exists in the backend, but v0.2.1 removed its UI
   action. There is therefore no explicit user-controlled handoff path.
5. Studio's context fitter can exceed its declared budget when the untrusted
   source/mention block itself is large.
6. Project-file audit findings:
   - ZIP import uses a lexical `starts_with` check that does not safely reject
     `..` archive entries.
   - Artifact paths read back from an imported project database are joined
     without re-applying the project sandbox boundary.
   - source re-ingest deletes `global_index` by `source_id` alone instead of the
     `(project_id, source_id)` ownership pair.

## Implementation

1. Drive PPTX rendering from loaded-presentation state, clamp empty/initial
   slide indexes, and add a regression test for the equal-slide-count case.
2. On successful Studio artifact import, invalidate source/artifact caches and
   open the new source in Viewer. Refresh Viewer detail/document/asset queries
   on matching source status events so MD, PDF, DOCX and PPTX move from queued
   to rendered without reopening the project.
3. Preserve v0.2.1's canonical `{t:"whole"}` request for automatic explanations
   and its one-thread-per-source query key; add persistence regression coverage
   proving the same key survives a page move without creating another thread.
4. Restore “引き継ぐ / Hand off” as an explicit mutation: await it, prevent
   duplicate in-flight clicks, invalidate Studio tabs, and show localized
   success/failure feedback. No Studio tab is created by opening, navigating,
   generating, or asking in Live Illustrator.
5. Bound untrusted Studio context before request assembly while preserving its
   boundary markers and newest conversation turn; add oversized-context tests.
6. Use ZIP enclosed paths and extraction ceilings, sandbox-resolve artifact
   files, scope global-index deletion by project, and add boundary regressions.

## Verification

- Frontend: typecheck, lint/design gates, unit tests, contrast, i18n parity,
  production build.
- Rust: fmt, clippy with warnings denied, all library/integration tests,
  generated binding drift, and cargo-deny.
- Targeted fixtures: Studio MD/PDF import lifecycle, PPTX equal-count render,
  source/page Live session identity, explicit-only Studio handoff, oversized AI
  context, ZIP traversal, artifact traversal, project-scoped index mutation.
- Manual rendered review in light/dark and normal/narrow layouts for Viewer,
  Studio workspace, and Live Illustrator drawer.

## Release and maintenance (last)

Set all package/bundle versions to 0.2.2; update README, release notes, quality
report, release draft, decisions and handoff; build and ad-hoc-sign the macOS
artifact; verify DMG/signature/startup/checksum; commit, tag and publish v0.2.2.
Then update the four-language Studio Rizi WAKARU page and the private
`common-rules-document/WAKARU.md`, running each repository's required gates.
