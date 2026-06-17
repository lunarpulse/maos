#!/usr/bin/env node
/**
 * gate:ko-coverage — Korean translation coverage reporter.
 *
 * NON-BLOCKING by default (exit 0): it makes the en↔ko coverage VISIBLE so the
 * Story 10.3 v1.0 ship-gate promise ("Korean translations present for all 5
 * canonical doc deliverables") is measured, not assumed. Story 9.5 deliberately
 * shipped a representative ko sample with English fallback (gate:fallback); this
 * reporter quantifies what remains.
 *
 * TEETH LATER: set KO_COVERAGE_MIN=<percent> to fail when canonical-section
 * coverage drops below it. Story 10.3 wires `KO_COVERAGE_MIN=100` into CI to turn
 * the promise into a mechanical gate (no tautological green — see the 9.5 review).
 *
 * Pairing mirrors gate-glossary-lock.js exactly (same en→ko path convention) so
 * the two never disagree about what "has a Korean counterpart" means.
 */
const fs = require("fs");
const path = require("path");

const SITE = path.join(__dirname, "..");
const KO_BASE = path.join(SITE, "i18n", "ko");

// Each Docusaurus content-docs plugin: en source dir → ko counterpart dir.
// Default plugin (docs/) and the versioned ABI plugin (id "abi", path abi/v1).
const PLUGINS = [
  {
    enDir: path.join(SITE, "docs"),
    koDir: path.join(KO_BASE, "docusaurus-plugin-content-docs", "current"),
  },
  {
    enDir: path.join(SITE, "abi", "v1"),
    koDir: path.join(KO_BASE, "docusaurus-plugin-content-docs-abi", "current"),
  },
];

// The five canonical doc deliverables + the generated ABI reference — the units
// Story 10.3's AC names. Coverage of THESE is what KO_COVERAGE_MIN gates on.
const CANONICAL = new Set([
  "manifest",
  "cookbook",
  "migrate",
  "troubleshoot",
  "deploy",
  "abi",
]);

function walkMdFiles(dir) {
  const results = [];
  if (!fs.existsSync(dir)) return results;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkMdFiles(full));
    } else if (entry.name.endsWith(".md") && !entry.name.startsWith("_")) {
      // Skip `_related_*.md` partials — prepended into pages, not routes of their own.
      results.push(full);
    }
  }
  return results;
}

function sectionOf(relPath) {
  const first = relPath.split(path.sep)[0];
  return first.endsWith(".md") ? "(root)" : first;
}

function main() {
  // section -> { total, translated }
  const sections = new Map();
  const bump = (name, translated) => {
    const s = sections.get(name) ?? { total: 0, translated: 0 };
    s.total += 1;
    if (translated) s.translated += 1;
    sections.set(name, s);
  };

  for (const { enDir, koDir } of PLUGINS) {
    for (const enFile of walkMdFiles(enDir)) {
      const relPath = path.relative(enDir, enFile);
      const koFile = path.join(koDir, relPath);
      // ABI pages all live flat under abi/v1 → section "abi"; default-plugin pages
      // are sectioned by their first path segment.
      const name = enDir.endsWith(path.join("abi", "v1")) ? "abi" : sectionOf(relPath);
      bump(name, fs.existsSync(koFile));
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

  console.log("gate:ko-coverage — Korean translation coverage (NON-BLOCKING report)\n");
  console.log("  Canonical doc deliverables (Story 10.3 v1.0 ship-gate target):");
  canonNames.forEach((n) => console.log(row(n)));
  const canon = sum(canonNames);
  console.log(
    `    ${"— canonical total".padEnd(16)} ${String(canon.translated).padStart(3)}/${String(canon.total).padEnd(4)} ${pct(canon.translated, canon.total)}\n`
  );

  console.log("  Supporting sections (English fallback acceptable — AC-4):");
  otherNames.forEach((n) => console.log(row(n)));
  const all = sum(names);
  console.log(
    `\n  Overall: ${all.translated}/${all.total} pages have a Korean counterpart (${pct(all.translated, all.total)}); the rest fall back to English.\n`
  );

  const minRaw = process.env.KO_COVERAGE_MIN;
  if (minRaw === undefined || minRaw === "") {
    console.log("Report-only (KO_COVERAGE_MIN unset). Set KO_COVERAGE_MIN=100 at Story 10.3 to enforce.");
    return;
  }
  const min = Number(minRaw);
  if (!Number.isFinite(min) || min < 0 || min > 100) {
    console.error(`FAIL: KO_COVERAGE_MIN must be a number 0-100, got: ${minRaw}`);
    process.exit(2);
  }
  const canonPct = canon.total === 0 ? 100 : (canon.translated / canon.total) * 100;
  if (canonPct < min) {
    console.error(
      `FAIL: canonical Korean coverage ${canonPct.toFixed(1)}% < required ${min}% (${canon.translated}/${canon.total} pages).`
    );
    process.exit(1);
  }
  console.log(`PASS: canonical Korean coverage ${canonPct.toFixed(1)}% >= required ${min}%.`);
}

main();
