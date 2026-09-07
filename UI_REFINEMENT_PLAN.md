# UI legibility refinement plan

Date: 2026-09-07

## Findings

- Decorative separators and interactive boundaries shared the same low-contrast
  hairline, so fields and panels became difficult to distinguish.
- Supporting type was used at 11–13 px across controls, tabs and metadata; the
  result was too small at the default display scale, especially in Japanese.
- SVGs used inline baseline layout and several one-off 12–16 px sizes, which made
  otherwise centred icon buttons appear optically misaligned.
- The existing hierarchy is sound. The correction should strengthen its visual
  grammar without moving screens or changing behavior.

## Hallmark-aligned correction

1. Keep 1 px structural separators, but make their surface contrast clearer.
2. Add a 2 px interactive-boundary token for controls, fields and the composer.
3. Raise the four smallest type steps by 1 px at the default scale and retain the
   existing responsive root scaling.
4. Render every application icon as a block-level, fixed flex item with one
   1.75-stroke voice; use 18 px for primary shell/navigation icons and keep
   compact icon controls at a readable 32 px box.
5. Verify light/dark contrast, 80–150% scaling behavior, narrow Settings layout,
   keyboard focus, automated tests and production build.

## Out of scope

- Navigation, page hierarchy, feature behavior, IPC and database contracts.
- New decoration, animation or a new palette.
