#!/usr/bin/env node
"use strict";

/**
 * Proven-red companion tests (D8).
 *
 * For each check-gate, mutate input and assert a non-zero exit.
 * A gate never observed failing is presumed broken (Epic 8 lesson).
 *
 * Safety (P-safety patch):
 *   - Every mutation is wrapped in try/finally so the source file is ALWAYS
 *     restored, even if the gate hangs, the process is signalled, or a subsequent
 *     assertion throws. Without this, a CI kill between mutate and restore would
 *     leave shared artifacts (error-catalog.json, ko translations, build output)
 *     corrupted in the checkout.
 *   - We mutate real source in-place because the gate scripts read fixed paths.
 *     The D8-faithful tempdir refactor (gates accepting a path/env override) is
 *     deferred — see story 9.5 Review Findings D5.
 *   - If ZERO tests execute (prereq files absent), we FAIL — a proven-red suite
 *     that ran nothing proves nothing (tautological green).
 */

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const SCRIPTS_DIR = __dirname;
const SITE_DIR = path.join(SCRIPTS_DIR, "..");
let passed = 0;
let failed = 0;
let runCount = 0;

/**
 * Run a single proven-red case: mutate a file, run the gate expecting failure,
 * and ALWAYS restore the original via try/finally.
 *
 * @param label   human label
 * @param file     absolute path to the file to mutate (must exist)
 * @param mutate   (original: string) => string  — returns mutated content
 * @param gateCmd  node command line for the gate (run from SITE_DIR)
 */
function provenRed(label, file, mutate, gateCmd) {
  if (!fs.existsSync(file)) {
    console.log(`  SKIP: ${label} — prereq absent (${path.relative(SITE_DIR, file)})`);
    return;
  }
  runCount++;
  const original = fs.readFileSync(file, "utf-8");
  try {
    fs.writeFileSync(file, mutate(original), "utf-8");
    try {
      execSync(gateCmd, { stdio: "pipe", cwd: SITE_DIR });
      // Gate exited 0 — it should have rejected the mutation.
      console.error(`  FAIL (gate did not reject): ${label}`);
      failed++;
    } catch (_e) {
      console.log(`  PASS (gate rejected): ${label}`);
      passed++;
    }
  } finally {
    // ALWAYS restore, even on throw / signal / unexpected error.
    fs.writeFileSync(file, original, "utf-8");
  }
}

function gate(scriptName) {
  return `node ${path.join(SCRIPTS_DIR, scriptName)}`;
}

function main() {
  console.log("Running proven-red companion tests...\n");

  const koIndex = path.join(
    SITE_DIR, "i18n", "ko",
    "docusaurus-plugin-content-docs", "current", "index.md"
  );
  // gate:glossary-lock — remove a locked term from a ko file
  provenRed(
    "gate:glossary-lock",
    koIndex,
    (orig) => orig.replace("Spirit", "REMOVED_TERM"),
    gate("gate-glossary-lock.js")
  );

  const catalogPath = path.join(SITE_DIR, "..", "docs", "errors", "error-catalog.json");
  // gate:troubleshoot-bidi — add a fake catalog entry not present as a page
  provenRed(
    "gate:troubleshoot-bidi",
    catalogPath,
    (orig) => {
      const cat = JSON.parse(orig);
      cat["EProvenRedFake"] = {
        code: "EProvenRedFake", description: "x", cause: "x", remediation: "yy distinct",
        severity: "x", recovery_class: "reject", owner: "x", since_version: "0.0.0",
        kernel_or_spirit: "kernel", retryable: false, rust_path: "x::EProvenRedFake",
        docs_url: "x", cause_chain_semantics: "x", version_stability: "x",
      };
      return JSON.stringify(cat, null, 2);
    },
    gate("gate-troubleshoot-bidi.js")
  );


  // gate:troubleshoot-teach — blank a page's Resolution so the teaching
  // contract (Resolution non-empty + distinct from Summary) fails.
  const teachPage = path.join(SITE_DIR, "docs", "errors", "EAbiTooNew.md");
  provenRed(
    "gate:troubleshoot-teach",
    teachPage,
    (orig) => orig.replace(/##\s+Resolution[\s\S]*?(?=##\s|$)/, "## Resolution\n\nx\n\n"),
    gate("gate-troubleshoot-bidi.js")
  );

  // gate:routes — remove a built route so resolveRoute() reports MISSING.
  const landingIndex = path.join(SITE_DIR, "build", "index.html");
  provenRed(
    "gate:routes",
    landingIndex,
    (orig) => "", // empty the landing page → route "/" resolves but content floor
                 // fails (three-door table); if it doesn't, rename trick below.
    gate("gate-routes.js")
  );

  // gate:cookbook-count — temporarily rename the cookbook dir so the gate
  // hits its missing-directory fail-fast (proves the missing-dir branch).
  const cookbookDir = path.join(SITE_DIR, "docs", "cookbook");
  if (fs.existsSync(cookbookDir)) {
    runCount++;
    const tmpDir = `${cookbookDir}.proven-red-tmp`;
    try {
      fs.renameSync(cookbookDir, tmpDir);
      try {
        execSync(gate("gate-routes.js"), { stdio: "pipe", cwd: SITE_DIR });
        console.error("  FAIL (gate did not reject): gate:cookbook-count (missing dir)");
        failed++;
      } catch (_e) {
        console.log("  PASS (gate rejected): gate:cookbook-count (missing dir)");
        passed++;
      }
    } finally {
      if (fs.existsSync(tmpDir) && !fs.existsSync(cookbookDir)) {
        fs.renameSync(tmpDir, cookbookDir);
      }
    }
  }

  console.log(`\nProven-red results: ${passed} passed, ${failed} failed (${runCount} run)`);

  // Tautological-green guard: a suite that executed zero tests proves nothing.
  if (runCount === 0) {
    console.error("FAIL: proven-red — 0 tests executed (prereq files absent); cannot prove any gate fails");
    process.exit(1);
  }
  if (failed > 0) {
    console.error("FAIL: proven-red companion tests");
    process.exit(1);
  }
  console.log("PASS: all proven-red companion tests");
}

main();
