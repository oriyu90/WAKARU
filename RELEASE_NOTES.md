# WAKARU v0.2.0

A feature and reliability release. It redesigns "資料を見る" around a vertical
tab rail, makes an opened document grow with the window, rebuilds the Live
Illustrator panel and has it explain the whole document on open, fixes
local-model (LM Studio / mlx-bar) replies not coming back — including a base URL
that omits `/v1` — lets you pick a model from the server's own list, keeps past
Studio conversations closed until you open them, stops a sent Studio message
lingering in the box, and tidies the window chrome. The project format, database
schema, and existing workflows are unchanged; projects from v0.0.0–v0.1.3 open
as-is with no migration.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## 資料を見る, redesigned

- **A vertical tab rail** replaces the row of horizontal tabs and the ladder of
  divider lines. The rail holds the source list, one row per open document (with
  a close button), and — pinned at the bottom — **追加**, **リンクを追加**, and
  the **ライブ解説** toggle.
- **An opened document fills the pane and re-fits as you resize the window.** PDF
  pages were drawn at their native size on a wide black margin; they now fit to
  width, and the − / % / + control multiplies on top of the fit.
- **Live Illustrator has a visible control.** The old open affordance was an
  8-pixel bar the same colour as the background. The rail button now turns the
  feature on (and opens the panel) when it is off, and opens/closes the panel
  when it is on. `Cmd/Ctrl + \` still toggles it. When the feature is on the
  panel opens by itself as soon as a document is open; turning it on from off
  opens the panel but waits for one tap ("解説をはじめる") before it runs.

## Live Illustrator, rebuilt

- **On open it explains the whole document**, not just page one — a plain-terms
  overview of what the document is for, how it is organised, and what to
  remember. A segmented control switches between 資料全体 and このページ.
- **The panel is simpler.** One header block instead of a stack of ruled rows:
  title, scope, detail level, then the explanation as the main content, the Q&A
  history collapsed by default, and a single ask box below.
- **It is separate from Studio.** The Live Illustrator conversation has always
  been its own thread; the "→ Studio" button is gone, so nothing it says is
  carried into a Studio tab.

## Local models

- **A base URL without `/v1` now works.** LM Studio, Ollama, llama.cpp, vLLM,
  mlx-bar — and `api.anthropic.com` itself — serve every route under `/v1`. A
  connection set to just `http://host:1234` sent requests to `/chat/completions`
  and `/embeddings`, which LM Studio answers with a bogus `200 "Unexpected
  endpoint"` (the reported "no reply"). WAKARU now completes a URL that has no
  path to `…/v1`; an explicit path you typed is left alone.
- **Pick a model from a list.** When the server exposes its catalogue
  (`GET /models`), the role rows in Settings offer the model ids as a dropdown
  with a "type it in" fallback, and the connection editor can fetch the list.
- **LM Studio and other OpenAI-compatible servers now return their replies.**
  WAKARU treated a stream that ended without the optional `data: [DONE]` marker
  as truncated and raised an error — but LM Studio in several setups, llama.cpp's
  server, and others close the stream cleanly right after the model finishes. A
  finished response is now recognised by its completion signal, with or without
  the marker. A genuine mid-stream disconnect is still reported.
- **Studio no longer fails on a model without tool support.** If the server
  rejects the request because it carried tool definitions, Studio retries the
  turn as a plain chat and keeps going without tools for the rest of that run.
- A streaming chat request is no longer re-sent on a temporary server error
  (which could start a second generation); only a failed connection is retried.

## Studio

- **Past conversations stay closed until you open them.** Studio no longer
  auto-opens the first tab and shows its history; it opens on a "pick a
  conversation" pane, and only the tab you click loads and displays.
- **A sent message clears from the composer immediately** (the turn is saved
  before the model is called, so it is never lost) — it no longer sits in the
  box next to its own sent bubble when a reply fails.
- The conversation list no longer loads every message of every tab on each
  refresh; only the open conversation's history is fetched. Long chat histories
  stay responsive.
- The selected tab is kept after sending, closing a tab, or reloading.

## Window

- **In fullscreen the toolbar no longer reserves empty space** where the macOS
  traffic lights would be — macOS hides them there. In a window the space is
  kept (the lights sit inside the top bar by design with this title-bar style)
  and the cluster is positioned to sit cleanly in the bar.

## Compatibility and limits

- Projects and settings from v0.0.0–v0.1.3 open unchanged. No database migration.
  The Studio tab list carries a message count instead of full histories (a call
  fetches one conversation on demand), and a new `ai_list_models` call lists a
  connection's models — neither changes a shared type. No project-format change,
  and the OpenAI/Anthropic wire formats are unchanged. A base URL you already
  store with an explicit path is untouched; one with no path gains `…/v1`.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
