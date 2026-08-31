#!/usr/bin/env node
/* Enforces the token discipline (docs/06 §2, Hallmark gate 48) + a few hard bans.
 * Run from `npm run lint`. Exit 1 on any violation. */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname } from "node:path";
import { fileURLToPath } from "node:url";

const SRC = fileURLToPath(new URL("../src/", import.meta.url));
const ALLOW_RAW = new Set([
  "styles/tokens.css", // the one place values are defined
  "styles/fonts.css",
  "styles/base.css", // the reset — hand-audited; carries the root font-size + clip pattern
]);
// The reset legitimately pairs `:focus { outline: none }` with `:focus-visible`.
const ALLOW_OUTLINE_NONE = new Set(["styles/base.css"]);

const RAW_COLOR = /(#[0-9a-fA-F]{3,8}\b|rgb\(|rgba\(|hsl\(|hsla\()/;
const RAW_PX = /(?<![\w-])-?\d*\.?\d+px\b/;
// px is allowed only for hairlines, radii, shadow offsets and filter radii.
const PX_OK_CONTEXT =
  /(border|outline|radius|box-shadow|text-shadow|translate|blur\(|saturate\(|inset:\s*-?\d)/i;

let violations = 0;
function fail(file, line, msg, text) {
  violations++;
  console.error(`  ${file}:${line}  ${msg}\n      ${text.trim()}`);
}

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (name === "__preview__" || name === "test") continue;
      walk(full);
      continue;
    }
    const ext = extname(name);
    if (![".ts", ".tsx", ".css"].includes(ext)) continue;
    const rel = full.slice(SRC.length);
    const src = readFileSync(full, "utf8");
    src.split("\n").forEach((text, i) => {
      const line = i + 1;
      if (
        /outline:\s*none/.test(text) &&
        !ALLOW_OUTLINE_NONE.has(rel) &&
        !/\/\*/.test(text.split("outline")[0])
      ) {
        fail(rel, line, "`outline: none` — a focus style must replace it", text);
      }
      // Monochrome (docs/07 §3, D-07): the #app `filter` reparents `position:
      // fixed` descendants, so the app uses `position: absolute` everywhere.
      if (/position:\s*fixed/.test(text) && !text.trimStart().startsWith("/*")) {
        fail(rel, line, "`position: fixed` is banned — monochrome filter breaks it (use absolute inside #app)", text);
      }
      if (ALLOW_RAW.has(rel)) return;
      if (ext === ".css") {
        if (RAW_COLOR.test(text) && !/var\(--/.test(text) && !text.trimStart().startsWith("/*") && !/oklch\(0% 0 0/.test(text)) {
          fail(rel, line, "raw colour in component CSS — use a token", text);
        }
        if (RAW_PX.test(text) && !PX_OK_CONTEXT.test(text) && !text.trimStart().startsWith("/*")) {
          fail(rel, line, "raw px — use a rem/space/token (px only for hairlines, radii, shadows)", text);
        }
      }
    });
  }
}

walk(SRC);

if (violations) {
  console.error(`\ndesign-rules: ${violations} violation(s)`);
  process.exit(1);
}
console.log("design-rules: ok");
