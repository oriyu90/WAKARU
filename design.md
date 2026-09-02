# Design — WAKARU

Locked design system for this app. Every screen reads this file before emitting code.
Do not regenerate per screen — amend this file when the system needs to grow.
`src/styles/tokens.css` is the machine source of truth; every value below is reproduced
there and `npm run check:contrast` recomputes all rendered pairs from it.

This system was designed from scratch for v0.2.0. It keeps the *disciplines* of
`一時保存/WAKARU/docs/06_UI仕様.md` (Hallmark rules, 8 states, `rem` everywhere, monochrome
safety, i18n, WCAG AA, density-first) but **not** its palette, fonts or per-screen pixel
layouts — those are re-derived here.

## System

- **Genre** · modern-minimal, application register (not a landing page).
- **Macrostructure family**
  - Shell / navigation · **Workbench** — dark-free, a thin top bar as the origin of the
    eye, a translucent overlay sidebar that never pushes layout, work surfaces separated
    by 1px rules and one step of paper lightness, never by stacked cards.
  - Reading surfaces (Viewer preview, Live Illustrator, reader view) · **Long Document** —
    a single measured serif column, generous leading, no chrome competing with the text.
  - Settings · **Workbench rail** — a left index of sections, a right editing surface;
    row label ↔ control correspondence beats decoration.
- **Theme** · custom OKLCH. Warm paper (anchor hue 70), one restrained ember accent
  (hue 40). Never cobalt-blue-everything. The accent is a highlighter, not a fill — it
  appears only on focus rings, the active nav item, citation marks, and the border+text of
  a primary action. Target ≤ 3 % of any viewport.
- **Axes** · warm near-white paper / roman grotesque display / ember accent.

## macOS-native register (v0.0.3)

The Hallmark identity is unchanged — warm paper, one ember accent (≤ 3 %), the 8-state
contract, `rem` everywhere, monochrome safety, WCAG AA, full JA/EN/zh i18n. On top of it
the app now reads as a native macOS application:

- **Type** · San Francisco via `-apple-system` as the primary UI face; `Geist Variable`
  stays bundled as the cross-platform / offline fallback (invariant I-2 intact).
- **Window** · overlay (hidden-inset) title bar (`titleBarStyle: "Overlay"`). The top bar is
  a **unified toolbar**: `-webkit-app-region: drag`, interactive controls `no-drag`, and a
  `--titlebar-inset-start` (4.75rem inside the shell, `--space-xs` in a browser) that clears
  the traffic lights.
- **Geometry** · control height 2rem (32px); radii 6 / 10 / 12; focus ring 3px accent.
- **Materials** · the overlay sidebar uses a stronger vibrancy (`blur(30px) saturate(180%)`)
  with an opaque `@supports` fallback.
- **Controls** · segmented control reads as `NSSegmentedControl` (pill-in-track, selected
  chip at paper with the whisper shadow); list rows read as Finder/Mail rows.
- Everything else in this file still governs; the register only tunes face, chrome and
  geometry.

## Tokens (canonical — `src/styles/tokens.css` is the source of truth)

```css
:root {
  /* Spacing · 4pt scale, named by role */
  --space-3xs: 0.125rem; --space-2xs: 0.25rem; --space-xs: 0.5rem;
  --space-sm: 0.75rem;   --space-md: 1rem;     --space-lg: 1.5rem;
  --space-xl: 2.5rem;     --space-2xl: 4rem;   --space-3xl: 6rem;

  /* Density — macOS-native control metrics (a document tool is dense on purpose) */
  --control-h: 2rem;         /* 32px @100% — standard macOS control height */
  --row-height: 1.875rem;    /* one list row */
  --tab-height: 2rem;
  --toolbar-height: 2.75rem; /* unified overlay toolbar — clears the traffic lights */
  --hit-min: 2.75rem;        /* hit-target floor; smaller controls expand via ::before */
  --titlebar-inset-start: var(--space-xs); /* → 4.75rem inside the Tauri shell (base.css) */
  --sidebar-width: 17rem;
  --content-measure: 66ch;

  /* Window-width fluid layout measures. `rem` keeps display-size scaling; the
     `vw` terms let wide pages centre (not pin to an edge) and let side rails
     collapse before the centre column. Breakpoints stay in `rem`. */
  --page-max: 66rem;          /* single-column page content cap — pages centre within it */
  --conversation-max: 48rem;  /* Studio / chat reading column (Claude-app shape) */
  --rail-w:  clamp(10.5rem, 15vw, 14rem);
  --panel-w: clamp(11rem, 21vw, 17rem);

  /* Rules, radii, rings — macOS geometry (6 / 10 / 12) */
  --rule-1: 1px;
  --radius-sm: 4px; --radius-md: 6px;  /* controls */ --radius-lg: 10px; /* panels, dialogs */
  --radius-pill: 999px;                /* chips only, never buttons */
  --focus-ring: 3px; --focus-offset: 2px;  /* macOS-weight accent ring */

  /* Elevation · one shadow, never stacked; only floating layers get it */
  --shadow-whisper: 0 1px 2px oklch(28% 0.02 70 / 0.06);
  --shadow-overlay: 0 10px 30px oklch(28% 0.02 70 / 0.16);

  /* Z-index · six named levels, no ad-hoc values */
  --z-base: 1; --z-raised: 10; --z-dropdown: 100;
  --z-sticky: 200; --z-modal: 400; --z-toast: 500; --z-tooltip: 600;

  /* Motion */
  --dur-micro: 120ms; --dur-short: 200ms; --dur-long: 380ms; --dur-reduced: 1ms;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-in: cubic-bezier(0.7, 0, 0.84, 0);
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);

  /* Typography · 2 + 1. macOS-native first (San Francisco via -apple-system);
     "Geist Variable" stays bundled as the offline fallback (invariant I-2). */
  --font-ui:   -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display",
               "Geist Variable", "Hiragino Kaku Gothic ProN", "Yu Gothic UI",
               "Noto Sans JP", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif;
  --font-read: "Spectral", "Hiragino Mincho ProN", "Yu Mincho",
               "Noto Serif JP", "Songti SC", Georgia, serif;
  --font-mono: "JetBrains Mono Variable", "SFMono-Regular", ui-monospace, monospace;

  --fs-2xs: 0.6875rem; --fs-xs: 0.75rem; --fs-sm: 0.8125rem; --fs-md: 0.9375rem;
  --fs-lg: 1.125rem;   --fs-xl: 1.375rem; --fs-2xl: 1.75rem;
  --lh-tight: 1.22; --lh-normal: 1.5; --lh-relaxed: 1.72;
  --tracking-tight: -0.02em; --tracking-normal: 0; --tracking-label: 0.06em;
}

/* Light — anchor hue 70 (warm oat). Not #fff, not flat grey. */
:root, :root[data-theme="light"] {
  --color-paper:        oklch(98.6% 0.006 70);
  --color-paper-2:      oklch(96.4% 0.008 70);   /* panels, sidebar */
  --color-paper-3:      oklch(93.2% 0.011 68);   /* sunken: inputs, rails */
  --color-rule:         oklch(85.5% 0.014 66);   /* decorative divider — visible */
  --color-rule-strong:  oklch(58%   0.017 64);   /* control boundary — clears 3:1 */
  --color-neutral:      oklch(56%   0.014 62);   /* icons, disabled text */
  --color-muted:        oklch(45%   0.014 60);   /* helper text — clears 4.5:1 */
  --color-ink:          oklch(32%   0.018 58);   /* body */
  --color-ink-strong:   oklch(22%   0.020 56);   /* headings — not #000 */

  --color-accent:       oklch(55%   0.152 40);   /* ember — single accent */
  --color-accent-ink:   oklch(99%   0.002 70);   /* text on an accent fill */
  --color-accent-weak:  oklch(94%   0.030 45);   /* selected-row wash */
  --color-focus:        oklch(52%   0.150 40);

  --color-success:      oklch(52%   0.115 150);
  --color-warning:      oklch(60%   0.120 75);
  --color-danger:       oklch(53%   0.180 25);

  --overlay-scrim:      oklch(28%   0.02 70 / 0.30);
  --overlay-panel:      oklch(96.6% 0.008 70 / 0.86);
  --topbar:             oklch(24%   0.022 60);
  --topbar-ink:         oklch(96%   0.006 70);
}

/* Dark — hue never moves; only lightness and chroma.
   ink / ink-strong / topbar-ink are EXACT white (checked by check:contrast);
   rules and muted text are lifted so panels and helper text stay legible. */
:root[data-theme="dark"] {
  --color-paper:        oklch(17%   0.014 66);
  --color-paper-2:      oklch(21.5% 0.015 66);   /* higher surface = lighter */
  --color-paper-3:      oklch(25.5% 0.016 66);
  --color-rule:         oklch(40%   0.016 66);   /* visible divider on 17% paper */
  --color-rule-strong:  oklch(61%   0.018 66);
  --color-neutral:      oklch(68%   0.013 64);
  --color-muted:        oklch(80%   0.011 62);
  --color-ink:          oklch(100%  0 0);
  --color-ink-strong:   oklch(100%  0 0);

  --color-accent:       oklch(72%   0.125 42);   /* +L, -C for dark */
  --color-accent-ink:   oklch(18%   0.02 60);
  --color-accent-weak:  oklch(30%   0.055 42);
  --color-focus:        oklch(76%   0.11 42);

  --color-success:      oklch(74%   0.115 150);
  --color-warning:      oklch(80%   0.115 75);
  --color-danger:       oklch(70%   0.150 25);

  --overlay-scrim:      oklch(10%   0.012 66 / 0.55);
  --overlay-panel:      oklch(21.5% 0.015 66 / 0.92);
  --topbar:             oklch(13%   0.014 66);
  --topbar-ink:         oklch(100%  0 0);
  --shadow-whisper: 0 1px 2px oklch(0% 0 0 / 0.40);
  --shadow-overlay: 0 10px 30px oklch(0% 0 0 / 0.55);
}
:root[data-theme="dark"] body { font-weight: 400; }
```

## Type

- **Display / UI** · Geist Variable, 400 body · 560 controls/labels · 640 headings.
  Tracking `-0.02em` on ≥ `--fs-lg`. Roman only — no italic headings (Hallmark ban).
- **Reading** · Spectral 400 / 500 / 600, italic 400 for in-prose emphasis. Used for
  Live Illustrator explanations, the web reader view, and long preview text. `--lh-relaxed`,
  measure `--content-measure`.
- **Mono / outlier** · JetBrains Mono Variable 500–700. Two roles only: keyboard shortcuts
  (`⌘K`) and machine strings (locators, token counts, code blocks). Never body.
- Size steps follow a ~1.2 ratio; no more than five sizes on one screen — hierarchy past
  that is weight and colour.
- Tabular numerals (`font-variant-numeric: tabular-nums`) on every count column.

## CTA voice

- **Primary** · paper ground, 1px `--color-accent` border, `--color-accent` text, weight
  560, `--radius-md`, padding `var(--space-xs) var(--space-md)`. Hover: `--color-accent-weak`
  wash. The accent is never a full button fill.
- **Secondary** · raised paper, 1px `--color-rule-strong` border, `--color-ink` text, same
  metrics.
- **Quiet** · no background; hover sinks to `--color-paper-2`. Used for row actions and
  icon buttons.
- Destructive keeps its confirm flow (name re-type for project delete); label carries a
  danger-coloured underline, not a red fill.

## Motion stance

- Animate `transform` and `opacity` only. Micro 120 ms (press, toggle, colour), short
  200 ms (hover, tooltip, menu), long 380 ms (drawer, dialog, sidebar).
- One orchestrated entrance on first paint of a screen; after that content is just there.
  No scroll-triggered reveals.
- Streaming text has no typewriter effect — token arrival is the motion.
- Focus rings appear instantly, never transitioned.
- `prefers-reduced-motion: reduce` → every transition to `--dur-reduced` (1 ms): state
  still changes, it just does not travel.

## Microinteractions stance

- Silent success. A save shows an inline "保存済み" state on the control, no toast.
- Toasts only for: failures, async work whose effect is off-screen, explicit confirmations.
  They stack at a corner with fixed positioning and never shift layout.
- Hover reveals nothing essential — every hover affordance has a focus/tap equivalent.
- Tooltip hover delay 800 ms, focus delay 0 ms.
- Reversible actions skip the confirm dialog; irreversible ones keep it.

## Monochrome (`FR-C4`, `docs/07 §3`)

- `:root[data-monochrome="true"] #app { filter: grayscale(1); }`. Because `filter`
  establishes a containing block, **nothing in this app may use `position: fixed`** — the
  sidebar, every dialog, every toast is `position: absolute` inside `#app`
  (`position: relative`).
- No state is signalled by colour alone. Every source kind, every status
  (`queued ○ · analyzing ◔ · ready ‹none› · ready_partial △ · failed !`), and every
  citation carries a glyph or shape. Waveforms are shape-coded, not colour-coded.
- Code highlighting switches to a weight/italic mono theme when monochrome is on.

## Display size (`FR-C5`)

`html { font-size }` alone scales the whole UI because every dimension is `rem`. Raw `px`
is allowed only for `--rule-1`, `--radius-*`, and shadow offsets. Steps: 80 / 90 / 100 /
110 / 125 / 150 %. 150 % at the 960×640 minimum window must not scroll horizontally — the
right-hand panels auto-collapse first.

## What every screen shares

- The dark top bar and the WAKARU wordmark (Geist 640, tracking `-0.02em`).
- The ember accent and its ≤ 3 % placement rule.
- Geist for UI, Spectral for reading bodies, JetBrains Mono for machine strings.
- Regions divided by 1px rules and one paper step — never overlapping cards, never a card
  inside a card.
- The 8-state contract on every interactive element (`docs/06 §9.1`).

## Per-screen allowances

- **Home** may show wider project cards; card interiors still carry a real hierarchy
  (name `--fs-lg` ink-strong, meta `--fs-2xs` muted mono).
- **Viewer** keeps its preview surface at the lightest paper; nothing tints it.
- **Studio** sinks its left/right rails one paper step below the centre conversation.
- **Settings** adds no ornament; label ↔ control alignment is the whole design.

## Stamp

```
/* Hallmark · genre: modern-minimal · macrostructure: Workbench · design-system: design.md · designed-as-app */
```
