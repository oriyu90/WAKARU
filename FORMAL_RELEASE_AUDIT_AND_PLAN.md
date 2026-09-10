# WAKARU formal-release audit and implementation plan

Status: implementation and release verification complete  
Target: `v1.0.0` (first formal stable release)  
Baseline audited: `v0.3.0` / `d443d62`

## Release decision

`v1.0.0` is appropriate only after the release gates below pass. The stable
label does not imply Apple notarisation or public repository access. The
current distribution remains Apple Silicon, macOS 12+, ad-hoc signed, not
notarised, and invite-only while the GitHub repository is private.

## Audit method

- Traced Viewer import/render, Studio RAG/tool execution, AI protocol adapters,
  PDF/DOCX generation, project archive boundaries, and static-site preview.
- Ran frontend type, lint/design, unit, build, contrast, i18n and production
  dependency audit gates; ran Rust unit/integration, clippy and dependency
  policy gates.
- Inspected the packaged macOS app in dark mode at 1170×768, including Home,
  source list, PDF controls, Live Illustrator, and Studio history.
- Inspected the four-language product site and its responsive/token system.

## Findings and disposition

### P0 — unsafe authored-site preview capability

The site preview iframe combined `allow-scripts` with `allow-same-origin` and
`allow-forms`, while its comment claimed an opaque origin. That combination is
an avoidable sandbox escape boundary for imported or model-authored HTML.

Decision: retain JavaScript previews but remove same-origin and form
capabilities. The frame gets an opaque origin, no popups, no top navigation,
and no referrer. This is the smallest compatible boundary.

### P1 — AI HTTP 200 failures looked like empty or truncated answers

The OpenAI-compatible adapter ignored malformed/non-SSE bodies and stream
parser failures. Servers such as LM Studio can return HTTP 200 with a plain
error body for a wrong route, so the UI could report a generic truncation or no
answer instead of the actual protocol failure.

Decision: require at least one valid SSE JSON event, detect a completed but
empty turn, and return `AI_BAD_RESPONSE` with an actionable reason. Preserve
valid LM Studio/MLXBar streams that omit `[DONE]` but provide a terminal
`finish_reason`.

### P1 — generated document format, filename and MIME could disagree

`build_document` trusted both `format` and `path`. A weak model could request
PDF bytes at `report.md`; artifact MIME inference also omitted PDF and DOCX.
The resulting file then failed or took the wrong path when imported into the
Viewer. Approval checks examined the unnormalised path, which could also miss
an overwrite at the effective filename.

Decision: make `format` authoritative, normalise the extension before both
approval and write, register PDF/DOCX MIME types, and add regression tests.

### P1 — document writes were crash-vulnerable

Studio wrote final artifacts directly to their destination. A process exit or
disk error could leave a truncated PDF/DOCX that was still discoverable by the
workspace.

Decision: write to a same-directory temporary file, flush and sync it, then
atomically persist it. The database row is updated only after the complete file
is in place.

### P2 — PDF toolbar collapsed into unreadable vertical text

With Live Illustrator open, the page controls were allowed to shrink. The
packaged app visibly rendered `1 / 1` as a vertical stack.

Decision: make the page-control group non-shrinking, prevent numeric labels
from wrapping, and keep horizontal overflow local to the toolbar.

### P2 — interactive document memory ceiling was too high

The old 200 MiB input ceiling did not account for ArrayBuffer copies, decoded
PDF canvases, Office DOM and renderer working memory. A valid large file could
create a multi-hundred-megabyte WebView spike.

Decision: cap interactive PDF/DOCX/PPTX rendering at 96 MiB. Ingestion,
extracted text, search and fallback reading remain available for larger files.

### P2 — product site no longer represented the application

The site described older UI work, hid the product behind an abstract evidence
card, and did not expose the Viewer → Live → Studio workflow clearly. The
download copy also needs to remain explicit about private distribution and
non-notarised builds.

Decision: rebuild the page around a product-like workbench, a short evidence
workflow, clear safety boundaries and a single honest download path. Keep
independent JA/EN/ZH/PT HTML, canonical/hreflang and JSON-LD.

## Areas found structurally sound

- Rust ownership and bounded parsers avoid unsafe code in the audited paths.
- Project ZIP extraction uses enclosed paths, file/byte ceilings and cleanup.
- Workspace and source paths are sandbox-resolved; symlinks are rejected or
  skipped at trust boundaries.
- API keys remain in the operating-system credential store and are excluded
  from application logs and project archives.
- RAG excerpts, tab history and tool output are bounded and explicitly marked
  as untrusted data.
- Live Illustrator sessions remain separate from Studio until the explicit
  handoff action; source-scoped sessions remain stable across page changes.

## Implementation order

1. Harden iframe, AI stream validation, document naming/MIME and atomic writes.
2. Fix Viewer compression and reduce document renderer memory exposure.
3. Rebuild the four-language website using one Hallmark token/layout system.
4. Add/extend regression tests and re-run every frontend and Rust gate.
5. Verify normal/narrow, light/dark app surfaces and normal/mobile website.
6. Bump to `v1.0.0`, build/sign/verify the DMG, publish a non-draft release,
   update Studio RIZI and the private common-rules maintenance record.

## Release gates

- Frontend: typecheck, lint/design/hardcoded, tests, i18n parity, contrast,
  production build and production dependency audit.
- Backend: fmt, clippy with warnings denied, all unit/integration tests,
  dependency policy, generated-binding drift check.
- Runtime: launch packaged app; Viewer, PDF correction controls, Studio history,
  Live handoff boundary and AI failure UX smoke tests; no secret-bearing logs.
- Artifact: version metadata, ad-hoc signature, DMG verification, mounted-app
  signature and launch, SHA-256, release asset re-download match.
- Website: site tests/build/validation/file-count, four locale URLs, canonical,
  structured data, release links, narrow/desktop visual QA.

## Deferred external decisions

- Making `oriyu90/WAKARU` public is a repository-governance decision and is not
  performed by this plan. The site must continue to say invite-only until the
  owner explicitly changes visibility.
- Developer ID signing and Apple notarisation require credentials not present
  in this workspace. The release must not imply either one.
- Live testing against a real external model requires an authorised endpoint
  and credential. Deterministic protocol contract tests remain mandatory even
  when no live endpoint is available.
