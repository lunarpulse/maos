#!/usr/bin/env node
"use strict";

/**
 * gate:glossary-lock — CI gate for Story 9.5 AC-3.
 *
 * Reads locked terms and denylist from LOCALES.md (D5).
 * Checks i18n/ko/ translation files against English sources.
 *
 * Checks:
 * 1. Per translation unit: count(term, ko) >= count(term, en)
 * 2. Canonical casing (case-sensitive)
 * 3. Denylist: known-bad translations must NOT appear in ko files
 */

const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..", "..");
const LOCALES_MD = path.join(ROOT, "LOCALES.md");
const EN_DOCS = path.join(__dirname, "..", "docs");
const KO_DOCS = path.join(
  __dirname, "..", "i18n", "ko",
  "docusaurus-plugin-content-docs", "current"
);

function parseLocalesMd(content) {
  const lockedTerms = [];
  const denylist = [];

  const ltMatch = content.match(
    /<!-- BEGIN LOCKED_TERMS -->([\s\S]*?)<!-- END LOCKED_TERMS -->/
  );
  if (ltMatch) {
    for (const line of ltMatch[1].split("\n")) {
      const trimmed = line.trim();
      if (trimmed && !trimmed.startsWith("<!--")) {
        lockedTerms.push(trimmed);
      }
    }
  }

  const dlMatch = content.match(
    /<!-- BEGIN DENYLIST -->([\s\S]*?)<!-- END DENYLIST -->/
  );
  if (dlMatch) {
    for (const line of dlMatch[1].split("\n")) {
      const trimmed = line.trim();
      if (
        trimmed.startsWith("|") &&
        !trimmed.startsWith("| English") &&
        !trimmed.startsWith("|---")
      ) {
        const cols = trimmed.split("|").map((c) => c.trim()).filter(Boolean);
        if (cols.length >= 2) {
          // Extract the Korean word before any parenthetical
          const badKorean = cols[1].split("(")[0].trim();
          if (badKorean) {
            denylist.push({ english: cols[0], badKorean });
          }
        }
      }
    }
  }

  return { lockedTerms, denylist };
}

function countOccurrences(text, term) {
  let count = 0;
  let pos = 0;
  while ((pos = text.indexOf(term, pos)) !== -1) {
    count++;
    pos += term.length;
  }
  return count;
}

function walkMdFiles(dir) {
  const results = [];
  if (!fs.existsSync(dir)) return results;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkMdFiles(full));
    } else if (entry.name.endsWith(".md")) {
      results.push(full);
    }
  }
  return results;
}

function main() {
  if (!fs.existsSync(LOCALES_MD)) {
    console.error("FAIL: LOCALES.md not found at", LOCALES_MD);
    process.exit(1);
  }
  const localesContent = fs.readFileSync(LOCALES_MD, "utf-8");
  const { lockedTerms, denylist } = parseLocalesMd(localesContent);

  console.log(
    `Loaded ${lockedTerms.length} locked terms, ${denylist.length} denylist entries`
  );

  const violations = [];

  const enFiles = walkMdFiles(EN_DOCS);
  console.log(`Scanning ${enFiles.length} English doc files`);

  let koFileCount = 0;
  for (const enFile of enFiles) {
    const relPath = path.relative(EN_DOCS, enFile);
    const koFile = path.join(KO_DOCS, relPath);

    // Skip if no Korean counterpart (fallback to English is OK per AC-4)
    if (!fs.existsSync(koFile)) continue;
    koFileCount++;

    const enContent = fs.readFileSync(enFile, "utf-8");
    const koContent = fs.readFileSync(koFile, "utf-8");

    // Check locked terms: count(term, ko) >= count(term, en)
    for (const term of lockedTerms) {
      const enCount = countOccurrences(enContent, term);
      const koCount = countOccurrences(koContent, term);
      if (enCount > 0 && koCount < enCount) {
        violations.push(
          `LOCKED_TERM: ${relPath}: "${term}" appears ${enCount}x in en but ${koCount}x in ko`
        );
      }
    }

    // Check denylist: bad Korean terms must NOT appear
    for (const { english, badKorean } of denylist) {
      if (koContent.includes(badKorean)) {
        violations.push(
          `DENYLIST: ${relPath}: found forbidden translation "${badKorean}" for "${english}"`
        );
      }
    }
  }

  console.log(`Checked ${koFileCount} ko translation files`);
  if (koFileCount === 0) {
    console.error(
      "FAIL: glossary-lock — 0 ko translation files checked (i18n/ko/ missing or empty); cannot prove any locked term is preserved"
    );
    process.exit(1);
  }

  if (violations.length > 0) {
    console.error(`\nFAIL: ${violations.length} glossary-lock violations:\n`);
    for (const v of violations) {
      console.error(`  - ${v}`);
    }
    process.exit(1);
  }

  console.log(
    "\nPASS: glossary-lock gate — all locked terms present, no denylist violations"
  );
}

main();
