#!/usr/bin/env node
"use strict";

/**
 * gate:routes — Verify all routes from the manifest resolve to real files
 * in the build output. Each must have a non-trivial body.
 *
 * gate:cookbook-count — Assert >=10 structural patterns (code fence + heading).
 */

const fs = require("fs");
const path = require("path");

const BUILD_DIR = path.join(__dirname, "..", "build");
const MANIFEST_PATH = path.join(__dirname, "..", "route-manifest.json");
const COOKBOOK_DIR = path.join(__dirname, "..", "docs", "cookbook");

function resolveRoute(route) {
  const clean = route.replace(/\/$/, "") || "";
  const candidates = [
    path.join(BUILD_DIR, clean, "index.html"),
    path.join(BUILD_DIR, `${clean}.html`),
  ];
  if (!clean) candidates.push(path.join(BUILD_DIR, "index.html"));
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return null;
}

function hasContentFloor(htmlPath, floor) {
  const html = fs.readFileSync(htmlPath, "utf-8");
  const bodyMatch = html.match(/<main[^>]*>([\s\S]*?)<\/main>/);
  if (!bodyMatch) return false;
  const body = bodyMatch[1];

  // Non-trivial: >100 chars of content
  const textContent = body.replace(/<[^>]+>/g, "").trim();
  if (textContent.length < 100) return false;

  if (!floor || floor === "prose") return true;
  if (floor === "code fence") {
    return /<code[\s>]/.test(body) || /<pre[\s>]/.test(body);
  }
  if (floor === "link list") return /<a\s[^>]*href/i.test(body);
  if (floor === "three-door table" || floor === "concepts table" || floor === "error table") {
    return /<table[\s>]/i.test(body);
  }
  if (floor === "pattern list" || floor === "module list") {
    return /<(ul|ol)[\s>]/i.test(body);
  }
  // Unknown floor type — non-triviality (>100 chars) already verified above.
  return true;
}

function main() {
  if (!fs.existsSync(BUILD_DIR)) {
    console.error("FAIL: build/ not found");
    process.exit(1);
  }

  const manifest = JSON.parse(fs.readFileSync(MANIFEST_PATH, "utf-8"));
  const routes = manifest.routes;
  const errorCodes = manifest.error_codes || [];

  let passed = 0;
  let failed = 0;
  const failures = [];

  // Check main routes
  for (const { path: route, label, content_floor } of routes) {
    const resolved = resolveRoute(route);
    if (!resolved) {
      failures.push(`MISSING: ${route} (${label})`);
      failed++;
      continue;
    }
    if (!hasContentFloor(resolved, content_floor)) {
      failures.push(`THIN: ${route} (${label}) — expected ${content_floor}`);
      failed++;
      continue;
    }
    passed++;
  }

  // Check error routes (also enforce content floor — error pages must not be stubs)
  for (const code of errorCodes) {
    const route = `/errors/${code}`;
    let resolved = resolveRoute(route);
    if (!resolved) {
      // Error routes with :: get URL-encoded; check filesystem
      const altCode = code.replace(/::/g, "-");
      resolved = resolveRoute(`/errors/${altCode}`);
      if (!resolved) {
        failures.push(`MISSING: ${route}`);
        failed++;
        continue;
      }
    }
    if (!hasContentFloor(resolved, "error table")) {
      failures.push(`THIN: ${route} — expected error table`);
      failed++;
      continue;
    }
    passed++;
  }

  console.log(`gate:routes — ${passed} passed, ${failed} failed`);
  if (failures.length > 0) {
    console.error("Failures:");
    for (const f of failures) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log("PASS: gate:routes");

  // gate:cookbook-count — >=10 structural patterns, verified against BUILT output
  // (Patch: a missing cookbook dir is the worst incompleteness — fail, don't skip.
  //  Patch: count patterns that actually shipped, not just authored source.)
  if (!fs.existsSync(COOKBOOK_DIR)) {
    console.error("FAIL: gate:cookbook-count — cookbook directory missing");
    process.exit(1);
  }
  const cookbookFiles = fs
    .readdirSync(COOKBOOK_DIR)
    .filter((f) => f.endsWith(".md") && f !== "index.md");
  let structuralCount = 0;
  const unbuilt = [];
  for (const file of cookbookFiles) {
    const srcContent = fs.readFileSync(path.join(COOKBOOK_DIR, file), "utf-8");
    const hasFence = /```/.test(srcContent);
    const hasHeading = /^## (?:Problem|Solution)/m.test(srcContent);
    if (!(hasFence && hasHeading)) continue;
    // Structural in source — now confirm it actually shipped in the build.
    const slug = file.replace(/\.md$/, "");
    const built = resolveRoute(`/cookbook/${slug}`);
    if (!built) {
      unbuilt.push(file);
      continue;
    }
    structuralCount++;
  }
  if (unbuilt.length > 0) {
    console.error(`FAIL: gate:cookbook-count — ${unbuilt.length} structural pattern(s) missing from build: ${unbuilt.join(", ")}`);
    process.exit(1);
  }
  console.log(`gate:cookbook-count — ${structuralCount} structural patterns (verified in build)`);
  if (structuralCount < 10) {
    console.error(`FAIL: need >=10, got ${structuralCount}`);
    process.exit(1);
  }
  console.log("PASS: gate:cookbook-count");
}

main();
