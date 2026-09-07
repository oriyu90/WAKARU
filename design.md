# Design — WAKARU

This is the locked visual contract for the application. `src/styles/tokens.css` is the
machine source of truth. Amend both files together when the system changes; screens must
not invent independent palettes or geometry.

## Direction

- **Genre:** quiet, conversational desktop workspace.
- **Reference:** contemporary conversation-first productivity interfaces: neutral
  surfaces, a soft sidebar, low-contrast separators, rounded controls, restrained
  hierarchy and a centred conversation composer. WAKARU uses no external brand assets.
- **Product hierarchy:** App shell → Home / File Modifier / Search / Settings → Project →
  Viewer / Studio. The visual remake does not alter this hierarchy.
- **Priority:** source readability, predictable controls and dense document work take
  precedence over decoration.

## Surfaces and colour

- The palette is neutral. Light mode uses white content with very light grey raised and
  sunken surfaces. Dark mode uses true black content with two charcoal surface steps.
- `--color-paper` is the content canvas, `--color-paper-2` is the sidebar or supporting
  rail, and `--color-paper-3` is selected/sunken content.
- `--color-accent` is the highest-contrast neutral: near-black in light mode and white in
  dark mode. It is used for the primary action and enabled switches.
- Semantic success, warning and danger colours remain available, always paired with a
  glyph or label so monochrome mode remains understandable.
- Text contrast is checked by `npm run check:contrast`; body pairs must clear 4.5:1 and
  component boundaries 3:1.

## Geometry and density

- Spacing uses the shared 4pt-derived `--space-*` scale.
- Standard controls and tabs are `2.25rem`; the title toolbar is `3.25rem`.
- Radii are 8 / 12 / 16px for small controls, normal controls and panels. Pills are
  reserved for chips, switches and circular actions.
- Borders are quiet and usually `--color-rule`. `--color-rule-strong` is reserved for
  boundaries that require 3:1 contrast.
- Elevation is limited to `--shadow-whisper` for floating controls and
  `--shadow-overlay` for dialogs and drawers. Shadows are never stacked.
- Every dimension except hairlines, radii and shadow offsets is expressed in `rem`.

## Application shell

- The top bar shares the active theme instead of becoming a separate dark brand strip.
  It remains draggable in Tauri and retains the macOS traffic-light inset.
- The sidebar remains an overlay by product requirement. It is `17rem` wide, uses the
  second surface step, and never pushes the current page. Navigation rows use a filled
  neutral selected state shared across the application.
- The shell keeps `Cmd/Ctrl+B`, `Esc`, focus trapping, focus restoration and screen-reader
  names.
- Settings keeps its own index rail; below `42rem` it becomes a horizontal scrolling
  section strip.

## Controls

- **Primary button:** solid highest-contrast neutral with inverse text, 12px radius.
- **Secondary button:** content surface, quiet border and whisper shadow.
- **Quiet button:** transparent until hover/focus.
- **Inputs:** raised grey surface, quiet border, 12px radius and a visible focus ring.
- **Tabs:** compact filled selection inside a neutral track; no bright underline.
- **Rows/cards:** prefer a change in surface tone over heavy borders. Project cards may
  keep their thin project-colour rail as an identifier, never as the main hierarchy.
- All controls preserve default, hover, focus-visible, active, disabled, loading, error
  and success states where those states apply.

## Studio

- Conversation history and workspace stay in supporting rails; the thread remains the
  lightest/lowest content canvas.
- All turns share `--conversation-max`. User turns use a compact neutral bubble aligned
  to the end; assistant turns remain open on the canvas for long-form readability.
- The composer is a single rounded, lightly elevated surface. The textarea blends into
  that surface, while the send action remains the strongest circular/rounded action.
- Tool approvals, citations, scopes and artifacts retain their existing behavior and
  semantic labels.

## Viewer and document surfaces

- Viewer hierarchy, source list, open-file tabs and Illustrator drawer remain unchanged.
- File tabs use the same neutral filled selection language as the rest of the app.
- Rendered documents stay on their natural document surface; the UI theme must not tint
  PDF/Office content.

## Typography

- UI: `-apple-system` / San Francisco first, Geist Variable as bundled fallback, followed
  by the existing Japanese and Chinese system fallbacks.
- Reading: Spectral with the existing CJK serif fallbacks. Users may switch to sans.
- Machine strings only: JetBrains Mono Variable.
- Headings are compact, roman and high contrast. Body copy is `--fs-md`; supporting labels
  may use `--fs-sm` or `--fs-xs`.

## Motion and accessibility

- Motion is limited to opacity, colour and transform. Sidebar/drawer motion uses
  `--dur-long`; controls use `--dur-micro` or `--dur-short`.
- `prefers-reduced-motion` collapses all animation durations to `--dur-reduced`.
- Focus is always visible and never communicated by colour alone.
- The whole UI remains usable in Japanese, English and Simplified Chinese, at display
  scales from 80–150%, in light/dark/monochrome modes and at the documented responsive
  breakpoints.

## Monochrome constraint

`:root[data-monochrome="true"] #app { filter: grayscale(1); }` is retained. Because a
filter changes the containing block for fixed descendants, application overlays use
`position: absolute` inside `#app`; `position: fixed` is prohibited by the design check.

## Canonical token summary

The complete values live in `src/styles/tokens.css`. The stable families are:

- spacing: `--space-3xs` through `--space-3xl`
- density: `--control-h`, `--row-height`, `--tab-height`, `--toolbar-height`
- measures: `--sidebar-width`, `--page-max`, `--conversation-max`, rails and panels
- geometry: `--rule-1`, `--radius-*`, `--focus-ring`, named shadows
- colours: `--color-paper*`, ink/muted/neutral/rules, semantic colours and overlays
- motion: named durations, easings and reduced-motion duration
- typography: UI, reading and mono families, named size/weight/leading tokens

## Source stamp

New shell or feature-level CSS may use:

```css
/* WAKARU · neutral conversational UI · design-system: design.md */
```
