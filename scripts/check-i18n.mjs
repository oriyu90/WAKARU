#!/usr/bin/env node
/* Every UI language must define exactly the same key set (docs/07 §2.2, AC-9-1). */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const langs = ["en", "ja", "zh-Hans"];
const dir = fileURLToPath(new URL("../src/i18n/", import.meta.url));

// i18next plural suffixes — ja/zh define only `_other` (docs/07 §2.2), so the
// parity check compares the base key, not the plural variants.
const PLURAL = /_(zero|one|two|few|many|other)$/;

function flatten(obj, prefix = "") {
  const out = [];
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object") out.push(...flatten(v, key));
    else out.push(key.replace(PLURAL, ""));
  }
  return out;
}

const keys = Object.fromEntries(
  langs.map((l) => [
    l,
    new Set(flatten(JSON.parse(readFileSync(`${dir}${l}.json`, "utf8")))),
  ]),
);

const base = keys.en;
let problems = 0;
for (const l of langs) {
  for (const k of base) {
    if (!keys[l].has(k)) {
      console.error(`  ${l}: missing "${k}"`);
      problems++;
    }
  }
  for (const k of keys[l]) {
    if (!base.has(k)) {
      console.error(`  ${l}: extra "${k}" (not in en)`);
      problems++;
    }
  }
}

if (problems) {
  console.error(`\ni18n: ${problems} problem(s)`);
  process.exit(1);
}
console.log(`i18n: ok (${base.size} keys × ${langs.length} languages)`);
