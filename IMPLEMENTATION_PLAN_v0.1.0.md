# WAKARU v0.1.0 implementation and verification plan

Date: 2026-09-07

## 1. Objective

Make the currently documented WAKARU feature set operate reliably, with special
attention to Live Illustrator, Studio, AI profile setup, ingestion, Viewer,
search, OCR, transcription, File Modifier, project portability, and failure
handling. Validate the AI-dependent paths against the owner-supplied
OpenAI-compatible endpoint and `Qwen3.8-27B-MLX-4bit` without storing its API key
in source, logs, artifacts, release notes, or site content.

Publish v0.1.0 only if implementation, design review, debugging, packaging, and
publication checks all pass.

## 2. Baseline analysis

- The v0.0.6 worktree starts clean and matches `origin/main`.
- Frontend typecheck, lint/design rules, 9 tests, contrast, 351-key localization
  parity, and the production web build pass.
- Rust formatting, clippy with warnings denied, 163 library tests, and 39 phase
  integration tests pass; one owner-authorized live-AI test is intentionally
  ignored by the default suite.
- The supplied endpoint responds to `/v1/models`, lists the requested model, and
  returns a valid non-streaming chat completion.
- The existing live integration harness is tied to an earlier endpoint/model by
  compile-time constants. That makes it unsuitable as a reusable acceptance test
  for the requested model and is the first confirmed remediation item.
- Existing project/database migrations, IPC contracts, local-first storage,
  keychain credential storage, source immutability, sandbox path checks, stream
  cancellation, bounded retries, and truncated-output detection must be preserved.
- The app UI currently supports Japanese, English, and Simplified Chinese. This
  release must keep Japanese and English complete and parity-checked; existing
  Simplified Chinese support must not regress.

## 3. Scope and acceptance matrix

### A. AI connection and capability discovery

- Make the live harness accept endpoint, model, embedding model, and timeout from
  environment variables while retaining safe defaults for manual development.
- Confirm model listing, reachability, streaming text, cancellation/truncation
  semantics, tool calls, vision probing, JSON-schema probing, and embeddings.
- Treat unsupported optional capabilities as explicit safe degradation rather
  than a crash or false success.

### B. Live Illustrator

- Verify page explanation, detail level, cache/regeneration, page/source/project
  question scopes, citations, persisted history, stop/cancel, and Studio import.
- Check Japanese and English prompts with grounded facts and a prompt-injection
  fixture.
- Reject empty, truncated, or unusable responses without caching them as success.

### C. Studio and generated files

- Verify source retrieval, tool-loop continuation, approval policy, sandbox path
  enforcement, cancellation, artifact persistence, artifact-to-source import,
  and Markdown/DOCX/PDF document creation where implemented.
- Ensure model output and generated files cannot expose the supplied credential.

### D. Local document workflows

- Re-run project CRUD/archive/delete confirmation, source ingestion and duplicate
  detection, failed-source recovery, Viewer tabs/locators, FTS fallback search,
  OCR, Whisper failure safety, image-to-PDF, text-to-MD/TXT, and ZIP
  export/import round trips.
- Add focused regression tests for any defect found during live or app testing.

### E. UI, localization, accessibility, and design quality

- Preserve the existing screen hierarchy and established design tokens.
- Review realistic Home, project/source list, Viewer + Illustrator, Studio,
  File Modifier, Search, Settings, dialog, loading, empty, and error states.
- Check light/dark themes, normal/narrow layouts, keyboard focus, readable control
  boundaries, icon alignment, and Japanese/English text fit according to the
  Hallmark UI quality rubric.
- Keep all user-visible strings localized; run parity and hardcoded-string gates.

### F. Safety and compatibility

- Do not introduce a database migration unless required. If required, test both a
  fresh database and upgrade from the current schema.
- Keep v0.0.0-v0.0.6 projects/settings importable and preserve original source
  files.
- Exercise malformed files, unreachable endpoint, authentication error, timeout,
  truncated SSE, unsupported capabilities, and cancellation without panic.
- Run clippy, dependency/license policy, path/sandbox tests, secret scans, and a
  release-build startup probe.

## 4. Implementation order

1. Generalize and strengthen the live endpoint test harness without embedding
   secrets or LAN-specific release behavior.
2. Run the requested model through capability, Illustrator, grounding,
   prompt-injection, Studio tool-loop, and artifact tests.
3. Diagnose each failure at the lowest responsible layer (transport/parser,
   service, command/IPC, state/UI), add a regression test, and implement the
   smallest compatibility-preserving fix.
4. Expand automated coverage for feature paths or failure modes that are only
   asserted informally today.
5. Build and launch the Tauri application, configure a disposable test profile,
   and inspect representative Japanese/English screens and interaction states.
6. Perform design evaluation and debugging passes; repeat all quality gates.
7. Update version metadata to 0.1.0, generated bindings/licenses when needed,
   README, release/quality documents, and private maintenance records.
8. Build the Apple Silicon macOS app and DMG with ad-hoc signing, verify code
   signatures, disk image integrity, mountability, launch, checksum, and absence
   of secrets.
9. Commit and push WAKARU, create tag/release `v0.1.0`, upload the verified DMG
   and checksum, re-download them, and compare size/hash.
10. Update the four localized Studio RIZI WAKARU pages, project/news metadata,
    canonical/structured data, sitemap date, and download links; run site tests,
    push, and verify the deployed URLs.
11. Record final commit, release, artifact, test, and site publication details in
    `common-rules-document/WAKARU.md`, then commit and push that repository.

## 5. Release blockers

Do not create v0.1.0 if any of the following remains:

- A documented core feature cannot complete its primary workflow.
- Live Illustrator or Studio fails with the supplied model for a WAKARU-supported
  capability that the endpoint advertises.
- A failure can crash the app, corrupt project data, escape the sandbox, leak the
  API key, or be displayed/cached as a false success.
- Japanese or English UI parity, contrast, keyboard focus, production build,
  Rust tests/clippy, license policy, package verification, or deployment checks
  fail.
- The release artifact cannot be reproduced, signed ad-hoc, mounted, launched,
  and checksum-verified.
