#!/usr/bin/env node
/**
 * gate:locale-coverage — Translation coverage reporter.
 *
 * Parameterized over LOCALE env var (default: "ko" for backward compat).
 * Set LOCALE=ja or LOCALE=zh-Hans for Japanese / Chinese Simplified.
 *
 * NON-BLOCKING by default (exit 0): makes the en↔locale coverage VISIBLE.
 * TEETH: set ${LOCALE_UPPER}_COVERAGE_MIN=<percent> (e.g. KO_COVERAGE_MIN=100)
 * to fail when canonical-section coverage drops below it.
 *
 * Story 10.3: Korean coverage gate (KO_COVERAGE_MIN=100).
 * Story 10.5 AC5: parameterized for ja/zh-Hans (JA_COVERAGE_MIN, ZHHANS_COVERAGE_MIN).
 */
const fs = require("fs");
const path = require("path");

const LOCALE = process.env.LOCALE || "ko";
const SITE = path.join(__dirname, "..");
const LOCALE_BASE = path.join(SITE, "i18n", LOCALE);

// Each Docusaurus content-docs plugin: en source dir → locale counterpart dir.
const PLUGINS = [
  {
    enDir: path.join(SITE, "docs"),
    localeDir: path.join(LOCALE_BASE, "docusaurus-plugin-content-docs", "current"),
  },
  {
    enDir: path.join(SITE, "abi", "v1"),
    localeDir: path.join(LOCALE_BASE, "docusaurus-plugin-content-docs-abi", "current"),
  },
];

// The five canonical doc deliverables + the generated ABI reference.
const CANONICAL = new Set([
  "manifest",
  "cookbook",
  "migrate",
  "troubleshoot",
  "deploy",
  "abi",
]);

function walkMdFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      // Skip errors/ directory (excluded from denominator per LOCALES.md:108-111)
      if (entry.name === "errors") continue;
      results.push(...walkMdFiles(full));
    } else if (entry.name.endsWith(".md")) {
      results.push(full);
    }
  }
  return results;
}

function sectionOf(relPath) {
  const first = relPath.split(path.sep)[0];
  return first.endsWith(".md") ? "(root)" : first;
}

// Resolve the coverage-min env var name for this locale.
function getCoverageMinEnvName() {
  // KO_COVERAGE_MIN, JA_COVERAGE_MIN, ZH_COVERAGE_MIN
  const key = LOCALE.replace(/-/g, "").toUpperCase() + "_COVERAGE_MIN";
  return key;
}

function main() {
  const sections = new Map();
  const bump = (name, translated) => {
    const s = sections.get(name) ?? { total: 0, translated: 0 };
    s.total += 1;
    if (translated) s.translated += 1;
    sections.set(name, s);
  };

  for (const { enDir, localeDir } of PLUGINS) {
    for (const enFile of walkMdFiles(enDir)) {
      const relPath = path.relative(enDir, enFile);
      const localeFile = path.join(localeDir, relPath);
      const name = enDir.endsWith(path.join("abi", "v1")) ? "abi" : sectionOf(relPath);
      bump(name, fs.existsSync(localeFile));
    }
  }

  const pct = (t, n) => (n === 0 ? "n/a" : `${Math.round((t / n) * 100)}%`);
  const names = [...sections.keys()].sort();
  const canonNames = names.filter((n) => CANONICAL.has(n));
  const otherNames = names.filter((n) => !CANONICAL.has(n));

  const sum = (list) =>
    list.reduce(
      (a, n) => {
        const s = sections.get(n);
        return { total: a.total + s.total, translated: a.translated + s.translated };
      },
      { total: 0, translated: 0 }
    );

  const row = (n) => {
    const s = sections.get(n);
    return `    ${n.padEnd(16)} ${String(s.translated).padStart(3)}/${String(s.total).padEnd(4)} ${pct(s.translated, s.total)}`;
  };

  const localeLabel = LOCALE.toUpperCase();
  console.log(`gate:locale-coverage — ${localeLabel} translation coverage\n`);
  console.log("  Canonical doc deliverables:");
  canonNames.forEach((n) => console.log(row(n)));
  const canon = sum(canonNames);
  console.log(
    `    ${"— canonical total".padEnd(16)} ${String(canon.translated).padStart(3)}/${String(canon.total).padEnd(4)} ${pct(canon.translated, canon.total)}\n`
  );

  console.log("  Supporting sections (English fallback acceptable):");
  otherNames.forEach((n) => console.log(row(n)));
  const all = sum(names);
  console.log(
    `\n  Overall: ${all.translated}/${all.total} pages have a ${localeLabel} counterpart (${pct(all.translated, all.total)}).\n`
  );

  const minEnvName = getCoverageMinEnvName();
  const minRaw = process.env[minEnvName];
  if (minRaw === undefined || minRaw === "") {
    console.log(`Report-only (${minEnvName} unset). Set ${minEnvName}=100 to enforce.`);
    return;
  }
  const min = Number(minRaw);
  if (!Number.isFinite(min) || min < 0 || min > 100) {
    console.error(`FAIL: ${minEnvName} must be a number 0-100, got: ${minRaw}`);
    process.exit(2);
  }
  const canonPct = canon.total === 0 ? 100 : (canon.translated / canon.total) * 100;
  if (canonPct < min) {
    console.error(
      `FAIL: canonical ${localeLabel} coverage ${canonPct.toFixed(1)}% < required ${min}% (${canon.translated}/${canon.total} pages).`
    );
    process.exit(1);
  }
  console.log(`PASS: canonical ${localeLabel} coverage ${canonPct.toFixed(1)}% >= required ${min}%.`);
}

main();
