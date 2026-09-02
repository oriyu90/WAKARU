# WAKARU v0.0.3

WAKARU v0.0.3 fixes local-network AI connections, makes the interface responsive
and readable, and restyles the app to feel native on macOS. Existing projects and
settings remain compatible — no database or settings migration.

From this release the **application source is public** (MIT). Earlier releases
kept only documentation in the public repository.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Highlights

- **Local-network AI connections** — the AI HTTP client no longer inherits a
  system or `*_PROXY` proxy, so a direct endpoint on the LAN or loopback
  (`http://192.168.x.x:1234/v1`, LM Studio, Ollama, a local MLX server) is
  reached directly. macOS Local Network access is now declared
  (`NSLocalNetworkUsageDescription`), so the app can prompt for and use it.
- **Real connection diagnostics** — a failed *Test* now reports the actual cause
  (connection refused / timeout / DNS / TLS) and a short checklist (base URL and
  port, server running, no intercepting proxy, macOS Local Network permission)
  instead of a generic "unreachable", and returns immediately without a retry
  delay. Localized in Japanese, English and Simplified Chinese.
- **Responsive, centred layout** — Home, Settings, Search and File Modifier
  centre their content within a fluid measure instead of pinning to one edge and
  leaving a wide void. Side rails and panels scale with the window; nothing
  clips at the right or bottom edge from 480 px to ultrawide, at 100 % and
  150 % display scale.
- **Studio conversation column** — Studio adopts a centred reading column with
  quiet, sunk side rails and visible column separators, so a running
  conversation stays comfortable to read at any window width.
- **Readable dark theme** — dividers, secondary text and panel separation are
  strengthened, and body text is set at full weight. Primary text stays exact
  white. Every rendered colour pair is re-verified against WCAG AA.
- **macOS-native styling** — San Francisco as the interface typeface (with the
  bundled fallback intact for other platforms), an overlay title bar with a
  draggable unified toolbar, macOS control geometry (32 px controls; 6 / 10 / 12
  corner radii; a 3 px accent focus ring), stronger sidebar vibrancy, and a
  native-style segmented control. The warm paper, single accent and eight
  interaction states are unchanged.

## Compatibility and limits

- macOS 12 or later on Apple Silicon. Windows and Linux are not built or
  verified in this release.
- Ad-hoc signed, not Apple-notarized.
- v0.0.0 / v0.0.1 / v0.0.2 projects and settings open unchanged; no schema
  migration.
- All feature limits documented for v0.0.2 (legacy binary Office formats,
  encrypted/corrupt documents, unsupported media codecs, scanned-PDF OCR
  indexing) still apply.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
