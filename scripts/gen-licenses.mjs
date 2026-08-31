#!/usr/bin/env node
/* Regenerate THIRD_PARTY_LICENSES.md from `cargo metadata` (docs/08 P11).
 * cargo-about isn't a hard dependency; this lists crate + version + SPDX id. */
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const meta = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--all-features"], {
    cwd: root + "src-tauri",
    maxBuffer: 64 * 1024 * 1024,
  }).toString(),
);

const pkgs = meta.packages
  .filter((p) => p.name !== "wakaru")
  .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.version.localeCompare(b.version));

const seen = new Set();
const rows = [];
for (const p of pkgs) {
  const key = `${p.name}@${p.version}`;
  if (seen.has(key)) continue;
  seen.add(key);
  const lic = p.license || (p.license_file ? `file: ${p.license_file}` : "see repository");
  rows.push(`| ${p.name} | ${p.version} | ${lic} |`);
}

const out = [
  "# Third-party licenses",
  "",
  "WAKARU itself is MIT-licensed (see LICENSE). The desktop binary statically links",
  "the Rust crates listed below. License identifiers are taken from each crate's",
  "Cargo manifest; full license texts ship inside each crate's source under",
  "`~/.cargo/registry/`. Regenerate with `npm run licenses:generate`.",
  "",
  "No crate here requires GPL/AGPL/LGPL terms. Where a crate offers a copyleft licence as one",
  "option in an `OR` expression (e.g. `r-efi`), WAKARU takes the permissive option. This is",
  "enforced by `cargo deny check licenses` against `src-tauri/deny.toml`.",
  "",
  "| Crate | Version | License |",
  "|---|---|---|",
  ...rows,
  "",
  `Total: ${seen.size} crates.`,
  "",
].join("\n");

writeFileSync(root + "THIRD_PARTY_LICENSES.md", out);
console.log(`THIRD_PARTY_LICENSES.md: ${seen.size} crates`);
