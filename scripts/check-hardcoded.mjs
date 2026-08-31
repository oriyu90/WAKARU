#!/usr/bin/env node
/* AC-9-1: no user-visible English hardcoded in the UI. Heuristic — flags JSX
 * text nodes with 2+ latin words that are not inside a {t(...)} call. Dev-only
 * screens (__preview__) and the design styleguide are exempt. */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOTS = ["app", "features"].map((d) =>
  fileURLToPath(new URL(`../src/${d}/`, import.meta.url)),
);

// Files where literal English is intentional (dev tooling / non-UI).
const EXEMPT = [/__preview__/, /\.test\.tsx?$/, /Icons\.tsx$/];

// JSX text node: `>  Two or more words here  <`
const JSX_TEXT = />[^<>{}]*?[A-Za-z]{2,}\s+[A-Za-z]{2,}[^<>{}]*</g;
// Allow tokens that are obviously not prose.
const OK = /^[\s>]*(var\(|https?:|—|·|\/|[0-9]|WAKARU|Studio|Serif|Sans|Geist|Spectral|JetBrains|OpenAI|LM Studio|Ollama|MIT|Apache|SIL OFL|API|URL|PDF|JSON|CSV|TSV|MCP|ID)/;

let violations = 0;
function walk(dir) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      walk(full);
      continue;
    }
    if (extname(name) !== ".tsx") continue;
    if (EXEMPT.some((re) => re.test(full))) continue;
    const src = readFileSync(full, "utf8");
    src.split("\n").forEach((line, i) => {
      if (line.includes("{t(") || line.includes("aria-label=") || line.includes("//")) return;
      for (const m of line.matchAll(JSX_TEXT)) {
        const text = m[0].slice(1, -1).trim();
        if (!text || OK.test(m[0]) || text.length < 6) continue;
        violations++;
        console.error(`  ${full.split("/src/")[1]}:${i + 1}  hardcoded: "${text}"`);
      }
    });
  }
}
ROOTS.forEach(walk);

if (violations) {
  console.error(`\nhardcoded-strings: ${violations} — wrap them in t(...)`);
  process.exit(1);
}
console.log("hardcoded-strings: ok");
