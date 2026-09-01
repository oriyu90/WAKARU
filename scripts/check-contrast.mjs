#!/usr/bin/env node
/* Recompute WCAG 2.1 contrast for every rendered (text, background) pair from
 * tokens.css — never trust the eye (docs/06 §1, design.md). Exit 1 on a miss. */
import { readFileSync } from "node:fs";

const css = readFileSync(new URL("../src/styles/tokens.css", import.meta.url), "utf8");

/* ---- OKLCH → linear sRGB → relative luminance ---- */
function oklchToLinearSrgb(L, C, H) {
  const hr = (H * Math.PI) / 180;
  const a = C * Math.cos(hr);
  const b = C * Math.sin(hr);
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ ** 3;
  const m = m_ ** 3;
  const s = s_ ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}
function luminance(L, C, H) {
  const [r, g, b] = oklchToLinearSrgb(L, C, H).map((v) => Math.max(0, Math.min(1, v)));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function ratio(fg, bg) {
  const a = luminance(...fg) + 0.05;
  const b = luminance(...bg) + 0.05;
  return a > b ? a / b : b / a;
}

function parseBlock(selector) {
  const start = css.indexOf(selector);
  if (start < 0) throw new Error(`block not found: ${selector}`);
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  const body = css.slice(open + 1, close);
  const map = {};
  for (const m of body.matchAll(/--([\w-]+):\s*oklch\(([\d.]+)%\s+([\d.]+)\s+([\d.]+)/g)) {
    map[m[1]] = [Number(m[2]) / 100, Number(m[3]), Number(m[4])];
  }
  return map;
}

const light = parseBlock(":root,\n:root[data-theme=\"light\"]");
const dark = parseBlock(':root[data-theme="dark"]');

// pair: [fg, bg, minimum]
const PAIRS = [
  ["color-ink", "color-paper", 4.5],
  ["color-ink", "color-paper-2", 4.5],
  ["color-ink-strong", "color-paper", 4.5],
  ["color-muted", "color-paper", 4.5],
  ["color-muted", "color-paper-3", 4.5],
  ["color-accent", "color-paper", 4.5], // links / primary CTA text
  ["color-accent-ink", "color-accent", 4.5], // text on an accent fill
  ["color-rule-strong", "color-paper", 3.0], // control boundary
  ["color-rule-strong", "color-paper-3", 3.0],
  ["color-focus", "color-paper", 3.0], // focus ring vs page
  ["color-danger", "color-paper", 4.5],
  ["topbar-ink", "topbar", 4.5],
];

let fails = 0;
for (const token of ["color-ink", "color-ink-strong", "topbar-ink"]) {
  if (JSON.stringify(dark[token]) !== JSON.stringify([1, 0, 0])) {
    fails++;
    console.error(`  [dark] ${token}: expected exact white`);
  }
}
for (const [name, map] of [
  ["light", light],
  ["dark", map_or_die(dark)],
]) {
  for (const [fg, bg, min] of PAIRS) {
    if (!map[fg] || !map[bg]) continue;
    const r = ratio(map[fg], map[bg]);
    const ok = r >= min;
    if (!ok) fails++;
    console.log(
      `  [${name}] ${fg} on ${bg}: ${r.toFixed(2)}:1  (min ${min})  ${ok ? "ok" : "FAIL"}`,
    );
  }
}

function map_or_die(m) {
  if (!m || Object.keys(m).length === 0) throw new Error("dark block empty");
  return m;
}

if (fails) {
  console.error(`\ncontrast: ${fails} pair(s) below minimum`);
  process.exit(1);
}
console.log("\ncontrast: ok");
