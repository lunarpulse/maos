#!/usr/bin/env node
"use strict";

/**
 * gate:a11y — rendered-DOM axe-core WCAG A/AA scan over distinct pages.
 *
 * The authoritative scan lives in Playwright (`tests/playwright/a11y.a11y.ts`)
 * so it can share the served-build lifecycle with the behavioral gates. Coverage
 * is deduped by rendered main-content fingerprint; Korean fallback pages are
 * counted in ko_translation_coverage but not double-counted as distinct pages.
 */

const { spawnSync } = require("child_process");

const result = spawnSync("npx", ["playwright", "test", "--project=a11y"], {
  cwd: `${__dirname}/..`,
  stdio: "inherit",
  shell: true,
});

process.exit(result.status ?? 1);
