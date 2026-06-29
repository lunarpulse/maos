#!/usr/bin/env node
"use strict";

/**
 * gate:glossary-lock — locale-invariant term gate.
 *
 * Reads locked terms and the per-locale denylist from LOCALES.md.
 * Parameterized by LOCALE (default: ko). Checks i18n/<locale>/ docs and ABI
 * translations against English sources.
 *
 * Checks:
 * 1. Per translation unit: count(term, locale) >= count(term, en)
 * 2. Canonical casing (case-sensitive)
 * 3. Per-locale denylist: known-bad translations must NOT appear
 */

const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..", "..");
const LOCALES_MD = path.join(ROOT, "LOCALES.md");
const LOCALE = process.env.LOCALE || "ko";
const EN_DOCS = path.join(__dirname, "..", "docs");
const LOCALE_DOCS = path.join(
  __dirname, "..", "i18n", LOCALE,
  "docusaurus-plugin-content-docs", "current"
);
const EN_ABI = path.join(__dirname, "..", "abi", "v1");
const LOCALE_ABI = path.join(
  __dirname, "..", "i18n", LOCALE,
  "docusaurus-plugin-content-docs-abi", "current"
);

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function denylistMarker(locale) {
  if (locale === "ko") return "DENYLIST";
  return `DENYLIST ${locale.toUpperCase()}`;
}

function parseLocalesMd(content, locale) {
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

  const marker = denylistMarker(locale);
  const dlRe = new RegExp(
    `<!-- BEGIN ${escapeRegExp(marker)} -->([\\s\\S]*?)<!-- END ${escapeRegExp(marker)} -->`
  );
  const dlMatch = content.match(dlRe);
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
          const badTerms = cols[1]
            .split("/")
            .map((term) => term.split("(")[0].trim())
            .filter(Boolean);
          for (const badTerm of badTerms) {
            denylist.push({ english: cols[0], badTerm });
          }
        }
      }
    }
  }

  return { lockedTerms, denylist, marker };
}

// A locked term counts only at identifier boundaries so "Spirit" is NOT
// satisfied by embedded occurrences in "SpiritVtable"/"SpiritId".
function isIdentChar(ch) {
  return /[A-Za-z0-9_-]/.test(ch);
}

function countOccurrences(text, term) {
  let count = 0;
  let pos = 0;
  while ((pos = text.indexOf(term, pos)) !== -1) {
    const before = pos > 0 ? text[pos - 1] : "";
    const after = pos + term.length < text.length ? text[pos + term.length] : "";
    if (!isIdentChar(before) && !isIdentChar(after)) {
      count++;
    }
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

function checkPair(enDir, translatedDir, label, lockedTerms, denylist) {
  const violations = [];
  const enFiles = walkMdFiles(enDir);
  let translatedFileCount = 0;
  for (const enFile of enFiles) {
    const relPath = path.relative(enDir, enFile);
    const translatedFile = path.join(translatedDir, relPath);
    if (!fs.existsSync(translatedFile)) continue;
    translatedFileCount++;
    const enContent = fs.readFileSync(enFile, "utf-8");
    const translatedContent = fs.readFileSync(translatedFile, "utf-8");
    for (const term of lockedTerms) {
      const enCount = countOccurrences(enContent, term);
      const translatedCount = countOccurrences(translatedContent, term);
      if (enCount > 0 && translatedCount < enCount) {
        violations.push(
          `LOCKED_TERM [${label} ${LOCALE}]: ${relPath}: "${term}" appears ${enCount}x in en but ${translatedCount}x in ${LOCALE}`
        );
      }
    }
    for (const { english, badTerm } of denylist) {
      if (translatedContent.includes(badTerm)) {
        violations.push(
          `DENYLIST [${label} ${LOCALE}]: ${relPath}: found forbidden translation "${badTerm}" for "${english}"`
        );
      }
    }
  }
  return { violations, translatedFileCount, enFileCount: enFiles.length };
}

function main() {
  if (!fs.existsSync(LOCALES_MD)) {
    console.error("FAIL: LOCALES.md not found at", LOCALES_MD);
    process.exit(1);
  }
  const localesContent = fs.readFileSync(LOCALES_MD, "utf-8");
  const { lockedTerms, denylist, marker } = parseLocalesMd(localesContent, LOCALE);

  console.log(
    `Loaded ${lockedTerms.length} locked terms, ${denylist.length} denylist entries from ${marker}`
  );

  const pairs = [
    { en: EN_DOCS, translated: LOCALE_DOCS, label: "docs" },
    { en: EN_ABI, translated: LOCALE_ABI, label: "abi" },
  ];

  const allViolations = [];
  let totalTranslatedFiles = 0;
  for (const p of pairs) {
    const { violations, translatedFileCount, enFileCount } = checkPair(
      p.en, p.translated, p.label, lockedTerms, denylist
    );
    console.log(`[${p.label}] scanned ${enFileCount} en files; checked ${translatedFileCount} ${LOCALE} counterparts`);
    allViolations.push(...violations);
    totalTranslatedFiles += translatedFileCount;
  }

  console.log(`Checked ${totalTranslatedFiles} ${LOCALE} translation files total`);
  if (totalTranslatedFiles === 0) {
    console.error(
      `FAIL: glossary-lock — 0 ${LOCALE} translation files checked (i18n/${LOCALE}/ missing or empty); cannot prove locked terms are preserved`
    );
    process.exit(1);
  }

  if (allViolations.length > 0) {
    console.error(`\nFAIL: ${allViolations.length} glossary-lock violations:\n`);
    for (const v of allViolations) {
      console.error(`  - ${v}`);
    }
    process.exit(1);
  }

  console.log(
    `\nPASS: ${LOCALE} glossary-lock gate — all locked terms present, no denylist violations`
  );
}

main();
