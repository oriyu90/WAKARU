# v0.0.1 release record

Implementation, automated verification, responsive browser testing, native
startup testing, and bundle verification are complete (2026-09-02) — see
`QUALITY_REPORT.md`. The verified DMG is
`src-tauri/target/release/bundle/dmg/WAKARU_0.0.1_aarch64.dmg`.

SHA-256:
`1b5f0d7200880f008cfbc4e9fde6b8c83629dd506ae7b8ec320cdcfcae32fe84`.

---

## 0. Repo situation

- Dev repo `適当/work/WAKARU` has **no git remote** — by design. App source stays
  local; `oriyu90/WAKARU` on GitHub is **public** and holds only
  README / LICENSE / release assets / site pointers ("source stays private").
- So there is nothing to `git push` for the source. The release publishes the
  **built DMG + docs**, not the code.
- The replacement changes are committed locally before release upload; the
  public repository must never receive this source tree.

---

## 1. Build (your Mac)

```bash
cd ~/places/適当/work/WAKARU
APPLE_SIGNING_IDENTITY="-" MACOSX_DEPLOYMENT_TARGET=12.0 npm run tauri build -- --bundles app
```

Output app: `src-tauri/target/release/bundle/macos/WAKARU.app`
(ad-hoc signed, **not notarized** — matches v0.1.0).

## 2. Verify the bundle

```bash
DMG=src-tauri/target/release/bundle/dmg/WAKARU_0.0.1_aarch64.dmg
hdiutil verify "$DMG"
MP=$(mktemp -d); hdiutil attach "$DMG" -mountpoint "$MP" -nobrowse
codesign --verify --deep --strict --verbose=2 "$MP/WAKARU.app"
hdiutil detach "$MP"
```

## 3. Manual smoke test — `docs/09 §8` (11 steps). Do not release if any step fails.

1. Cold-start the app (note the time).
2. Create a project.
3. Drop in fixtures at once: a PDF, an image, an mp4, an xlsx.
4. While analyzing — page through / search (must not freeze).
5. Open the PDF, run Live Illustrator on 3 pages, ask a question.
6. Quit and relaunch → tabs and conversations restored.
7. In Studio, make a summary and add it to sources.
8. Turn on monochrome, set display size 150%, switch language to 中文 — walk the app.
9. Disconnect the network — viewing, search, export still work.
10. Export a project → delete it → import it → identical.
11. Open the log — no secrets leaked.

## 4. Checksums

```bash
cd src-tauri/target/release/bundle/dmg
shasum -a 256 WAKARU_0.0.1_aarch64.dmg
```

## 5. Owner confirmation

Run the app once end-to-end and confirm it's good. (Plan Part 4 step 6 /
`docs/最後にやって欲しいこと…`.)

---

## 6. Create or replace the GitHub release

```bash
cd ~/places/適当/work/WAKARU
gh release upload v0.0.1 --repo oriyu90/WAKARU --clobber \
  src-tauri/target/release/bundle/dmg/WAKARU_0.0.1_aarch64.dmg \
  src-tauri/target/release/bundle/dmg/WAKARU_0.0.1_aarch64.dmg.sha256
gh release edit v0.0.1 --repo oriyu90/WAKARU \
  --title "WAKARU v0.0.1" --notes-file RELEASE_NOTES.md --prerelease=false --latest
```

For a new tag, use `gh release create` with the same assets and `--latest`.

**Release body** = `RELEASE_NOTES.md` (this repo). v0.0.0 and v0.0.1 are
正式Release; v0.0.1 is explicitly marked Latest.

---

## 7. LAST — docs + site (only after the release is live; common-rules rule 1)

### 7a. Public repo `oriyu90/WAKARU` (keep it source-free)

Update / add, commit as `Yuki_Orita`:

- `README.md` — features & requirements to match reality (see draft below).
- `RELEASE_NOTES.md` — copy this repo's `RELEASE_NOTES.md`.
- `QUALITY_REPORT.md` — copy this repo's `QUALITY_REPORT.md` after the manual
  gates are filled in.
- `THIRD_PARTY_LICENSES.md` — copy this repo's (regenerated) file.

### 7b. `oriyu90/studio-rizi` — `website/projects/wakaru/` (+ `en/ pt/ zh/`)

Per `STUDIO-RIZI-PROJECT-HOSTING.md`: update the feature list and requirements;
version normally stays out of the body (the page links "latest release"). Then
at repo root:

```bash
npm run build && npm run validate && npm run count-files
```

Commit as `Yuki <yukiseno0911@gmail.com>`.

### 7c. Private `common-rules-document/WAKARU.md`

Bump: current version → `0.0.1`, scope note (full re-development, Tauri v2 +
Rust + Hallmark React), open TODOs (PDF raster D-09, local embeddings D-10,
Vision D-11, Studio streaming D-13, MCP HTTP D-14, Silero/keyframes D-15,
Win/Linux unverified).

---

## Draft: public `README.md` for `oriyu90/WAKARU`

```markdown
# WAKARU

A local-first AI assistant for your own documents. Ingest PDFs, images, audio,
video, spreadsheets and web links; get the page you're reading explained with
citations; and work from your sources in a chat workspace with tools.

Works with any OpenAI-compatible endpoint — cloud APIs, LM Studio, Ollama.
Nothing leaves your machine except the requests you make to the AI endpoints
you configure.

## Install (macOS, Apple Silicon)

Download `WAKARU_0.0.1_aarch64.dmg` from the [v0.0.1 release](../../releases/tag/v0.0.1).
The app is ad-hoc signed and **not notarized** — on first launch, right-click
it and choose **Open** to get past Gatekeeper.

Verify the download:

    shasum -a 256 WAKARU_0.0.1_aarch64.dmg
    # compare against RELEASE_CHECKSUMS.txt in the release

## Requirements

- macOS 12+ on Apple Silicon.
- An OpenAI-compatible or Anthropic-compatible endpoint for AI features (optional — ingestion, viewing
  and keyword search work offline).
- For audio/video transcription: download a Whisper model in Settings
  (~150 MB–3 GB depending on size).

## What it does

- **Projects & ingestion** — PDF text, Office formats, images, audio/video
  transcription (whisper.cpp), web links with an SSRF guard, CJK-aware
  full-text search.
- **Live Illustrator** — cached, cited explanations of the page you're on.
- **Studio** — chat over your sources with built-in tools, a `workspace/`
  folder, artifact cards, and optional local MCP servers.
- **Sandbox** — everything the model can write or run is confined to
  `workspace/`: no shell, scrubbed environment, path-traversal blocked.
- **File Modifier** — images → PDF, paste → organised Markdown.
- English / 日本語 / 简体中文, light/dark/system, monochrome mode.

## Privacy

- AI/API keys are stored in the macOS keychain, never on disk in plain text.
- The app only connects to: the AI / MCP endpoints you register, web links you
  explicitly add, Whisper model downloads you start, and the update check
  (which can be turned off).

## Limitations (v0.0.1)

- Windows / Linux builds are unverified and not distributed.
- PDF/slide pages show extracted text, not rendered images.
- Local embedding models and ingest-time image analysis are not included yet.
- See the release notes for the full list.

## License

MIT © Yuki Orita. Third-party components: see `THIRD_PARTY_LICENSES.md`.
```
