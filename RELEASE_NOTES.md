# WAKARU v0.0.6

WAKARU v0.0.6 improves the legibility and alignment of the refreshed interface
without changing the application hierarchy, workflows or data formats.

This build is for macOS Apple Silicon. It is ad-hoc signed and **not notarized**;
on first launch, right-click WAKARU and choose **Open**.

## Highlights

- **Clearer control boundaries** — important controls and input surfaces use stronger
  outlines and separators in both light and dark themes.
- **More comfortable sizing** — standard controls are now 40 px high, body text is
  16 px, and supporting labels follow a consistent 12/13/14 px scale.
- **Better icon alignment** — icon sizing, stroke weight and SVG layout are unified
  so symbols sit centrally inside buttons and rows.
- **Steadier page rhythm** — empty states, headings and supporting copy align more
  consistently with their content columns.
- **Accessibility preserved** — keyboard focus, contrast, display scaling,
  monochrome mode and reduced-motion behavior remain supported.

## Compatibility and limits

- Existing v0.0.0–v0.0.5 projects and settings open unchanged. There is no database
  migration in this release.
- OCR, document generation, Viewer rendering, search, local/LAN model connections,
  MCP and all other capabilities are unchanged.
- macOS 12 or later on Apple Silicon. Windows and Linux are not built or verified in
  this release.
- Ad-hoc signed, not Apple-notarized.

See `QUALITY_REPORT.md` for the verification record and artifact checksum.
