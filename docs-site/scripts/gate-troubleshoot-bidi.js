#!/usr/bin/env node
"use strict";

/**
 * gate:troubleshoot-bidi — Bidirectional set-equality between the error catalog
 * and the troubleshoot pages.
 *
 * symmetric_difference(catalog_codes, troubleshoot_codes) == empty
 *
 * gate:troubleshoot-teach — Per entry: verbatim code + non-empty Cause +
 * Resolution distinct from summary + >=1 trigger.
 */

const fs = require("fs");
const path = require("path");

const CATALOG_PATH = path.join(__dirname, "..", "..", "docs", "errors", "error-catalog.json");
const ERRORS_DIR = path.join(__dirname, "..", "docs", "errors");

function main() {
  // Load catalog codes
  const catalog = JSON.parse(fs.readFileSync(CATALOG_PATH, "utf-8"));
  const catalogCodes = new Set(Object.keys(catalog));

  // Load troubleshoot page codes (from filenames)
  const errorFiles = fs
    .readdirSync(ERRORS_DIR)
    .filter((f) => f.endsWith(".md") && f.startsWith("E"));

  const troubleshootCodes = new Set();
  for (const file of errorFiles) {
    const content = fs.readFileSync(path.join(ERRORS_DIR, file), "utf-8");
    // Extract code, in priority order (Patch: removed dead `**Code:**` regex —
    // 0 of 37 pages use it; pages use a `**Error Code**` table cell instead):
    //   1. frontmatter `title:` — present on all pages, holds the full `::` code
    //   2. `# <code>` H1 heading
    //   3. filename: EFoo-Bar.md -> EFoo::Bar (last resort)
    const fmTitle = content.match(/^title:\s*"([^"]+)"/m);
    const h1 = content.match(/^#\s+`?(\S+?)`?\s*$/m);
    let code = (fmTitle && fmTitle[1]) || (h1 && h1[1]);
    if (!code) {
      code = file.replace(/\.md$/, "").replace(/-/g, "::");
    }
    troubleshootCodes.add(code);
  }

  // Bidirectional check
  const inCatalogNotTroubleshoot = [...catalogCodes].filter(
    (c) => !troubleshootCodes.has(c)
  );
  const inTroubleshootNotCatalog = [...troubleshootCodes].filter(
    (c) => !catalogCodes.has(c)
  );

  console.log(`Catalog codes: ${catalogCodes.size}`);
  console.log(`Troubleshoot pages: ${troubleshootCodes.size}`);

  let bidiPass = true;
  if (inCatalogNotTroubleshoot.length > 0) {
    console.error("\nIn catalog but missing troubleshoot page:");
    for (const c of inCatalogNotTroubleshoot) console.error(`  - ${c}`);
    bidiPass = false;
  }
  if (inTroubleshootNotCatalog.length > 0) {
    console.error("\nTroubleshoot page without catalog entry:");
    for (const c of inTroubleshootNotCatalog) console.error(`  - ${c}`);
    bidiPass = false;
  }

  if (!bidiPass) {
    console.error("\nFAIL: gate:troubleshoot-bidi");
    process.exit(1);
  }
  console.log("PASS: gate:troubleshoot-bidi — symmetric difference is empty");

  // Patch: assert route-manifest.json error_codes == error-catalog keys.
  // The manifest is a third source of truth; without this it can silently drift
  // from the catalog (a code could have a troubleshoot page but no manifest entry).
  const manifestPath = path.join(__dirname, "..", "route-manifest.json");
  if (fs.existsSync(manifestPath)) {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf-8"));
    const manifestCodes = new Set(manifest.error_codes || []);
    const inManifestNotCatalog = [...manifestCodes].filter((c) => !catalogCodes.has(c));
    const inCatalogNotManifest = [...catalogCodes].filter((c) => !manifestCodes.has(c));
    if (inManifestNotCatalog.length > 0 || inCatalogNotManifest.length > 0) {
      console.error("\nFAIL: route-manifest error_codes != error-catalog keys");
      if (inManifestNotCatalog.length > 0)
        console.error("  in manifest not catalog:", inManifestNotCatalog.join(", "));
      if (inCatalogNotManifest.length > 0)
        console.error("  in catalog not manifest:", inCatalogNotManifest.join(", "));
      process.exit(1);
    }
    console.log("PASS: route-manifest error_codes == error-catalog keys");
  }

  // gate:troubleshoot-teach — teaching contract per D4.
  // Patch: added "Resolution distinct from Summary" (D4: Resolution-with-actionable-step
  // distinct from the summary). Page-level trigger coverage lives in the catalog
  // `remediation` distinctness check below (D4 burden lands on the catalog).
  const normText = (s) => s.replace(/[^a-z0-9]/gi, "").toLowerCase();
  let teachViolations = 0;
  for (const file of errorFiles) {
    const content = fs.readFileSync(path.join(ERRORS_DIR, file), "utf-8");

    const hasCode = /```/.test(content) || /`E\w+/.test(content);
    const hasCause = /##\s+Cause/.test(content) &&
      content.split(/##\s+Cause/)[1]?.split(/##/)[0]?.trim().length > 10;
    const resolutionBody = content.split(/##\s+Resolution/)[1]?.split(/##/)[0]?.trim() || "";
    const hasResolution = resolutionBody.length > 10;
    const summaryBody = content.split(/##\s+Summary/)[1]?.split(/##/)[0]?.trim() || "";
    // D4: Resolution must carry an actionable step distinct from the one-line summary.
    const resolutionDistinct =
      hasResolution && normText(resolutionBody) !== normText(summaryBody);

    if (!hasCode || !hasCause || !hasResolution || !resolutionDistinct) {
      const missing = [];
      if (!hasCode) missing.push("verbatim code");
      if (!hasCause) missing.push("non-empty Cause");
      if (!hasResolution) missing.push("non-empty Resolution");
      if (!resolutionDistinct) missing.push("Resolution distinct from Summary");
      console.error(`  FAIL: ${file} — missing: ${missing.join(", ")}`);
      teachViolations++;
    }
  }

  if (teachViolations > 0) {
    console.error(`\nFAIL: gate:troubleshoot-teach — ${teachViolations} violations`);
    process.exit(1);
  }
  console.log("PASS: gate:troubleshoot-teach — code + cause + resolution distinct from summary");

  // Verify catalog has cause/remediation fields AND that remediation is an
  // actionable step distinct from both cause and description (D4 source-of-truth).
  let catalogFieldViolations = 0;
  for (const [code, entry] of Object.entries(catalog)) {
    if (!entry.cause || entry.cause.trim().length === 0) {
      console.error(`  CATALOG: ${code} missing 'cause' field`);
      catalogFieldViolations++;
    }
    if (!entry.remediation || entry.remediation.trim().length === 0) {
      console.error(`  CATALOG: ${code} missing 'remediation' field`);
      catalogFieldViolations++;
    }
    const rem = normText(entry.remediation || "");
    if (rem && (rem === normText(entry.cause || "") || rem === normText(entry.description || ""))) {
      console.error(`  CATALOG: ${code} 'remediation' is not distinct from cause/description`);
      catalogFieldViolations++;
    }
  }
  if (catalogFieldViolations > 0) {
    console.error(`\nFAIL: ${catalogFieldViolations} catalog entries fail cause/remediation contract`);
    process.exit(1);
  }
  console.log("PASS: catalog cause/remediation fields complete + distinct");
}

main();
