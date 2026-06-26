#!/usr/bin/env node
"use strict";

/**
 * gate:anchor-ids — ABI citable headings must carry explicit locale-invariant IDs.
 *
 * D-cons4: Docusaurus auto-slugs headings from rendered text; translated headings
 * drift and break permanent /abi/v1/* deep links. The ABI docs are the citable
 * contract, so every heading in generated ABI pages must include `{#id}`.
 */

const fs = require("fs");
const path = require("path");

const SITE_DIR = path.join(__dirname, "..");
const ABI_DIRS = [
  path.join(SITE_DIR, "abi", "v1"),
  path.join(
    SITE_DIR,
    "i18n",
    "ko",
    "docusaurus-plugin-content-docs-abi",
    "current"
  ),
  path.join(
    SITE_DIR,
    "i18n",
    "ja",
    "docusaurus-plugin-content-docs-abi",
    "current"
  ),
  path.join(
    SITE_DIR,
    "i18n",
    "zh-Hans",
    "docusaurus-plugin-content-docs-abi",
    "current"
  ),
];

function markdownFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  return fs
    .readdirSync(dir)
    .filter((name) => name.endsWith(".md") && !name.startsWith("_related_"))
    .map((name) => path.join(dir, name));
}

function isFence(line) {
  const trimmed = line.trimStart();
  return (trimmed.startsWith("```") || trimmed.startsWith("~~~")) && /^[`~]{3,}/.test(trimmed);
}

function fenceLength(line) {
  const match = /^[`~]+/.exec(line.trimStart());
  return match ? match[0].length : 0;
}

function headingMissingExplicitId(line) {
  const trimmed = line.trimStart();
  if (!trimmed.startsWith("#")) return false;
  const match = /^(#{1,6})\s+(.+)$/.exec(trimmed);
  if (!match) return false;
  return !/\s\{#[A-Za-z0-9_-]+\}\s*$/.test(match[2]);
}

function checkFile(file) {
  const content = fs.readFileSync(file, "utf-8");
  const failures = [];
  let fenceOpen = null; // { char: '`'|'~', length: number }
  content.split(/\r?\n/).forEach((line, idx) => {
    if (isFence(line)) {
      const len = fenceLength(line);
      const char = line.trimStart()[0];
      if (fenceOpen && fenceOpen.char === char && fenceOpen.length === len) {
        fenceOpen = null;
      } else if (!fenceOpen) {
        fenceOpen = { char, length: len };
      }
      return;
    }
    if (!fenceOpen && headingMissingExplicitId(line)) {
      failures.push(`${path.relative(SITE_DIR, file)}:${idx + 1}: ${line}`);
    }
  });
  return failures;
}

function main() {
  const files = ABI_DIRS.flatMap(markdownFiles);
  if (files.length === 0) {
    console.error("FAIL: gate:anchor-ids — no ABI markdown files found");
    process.exit(1);
  }

  const failures = files.flatMap(checkFile);
  if (failures.length > 0) {
    console.error("FAIL: gate:anchor-ids — citable ABI headings missing explicit IDs:");
    for (const failure of failures) console.error(`  - ${failure}`);
    process.exit(1);
  }

  console.log(`PASS: gate:anchor-ids — ${files.length} ABI markdown page(s) have explicit heading IDs`);
}

main();
