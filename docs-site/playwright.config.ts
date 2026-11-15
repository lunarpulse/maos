import { defineConfig, devices } from "@playwright/test";

function requirePort(): number {
  const raw = process.env.DOCS_SITE_PORT ?? "4173";
  const port = Number(raw);
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    throw new Error(`DOCS_SITE_PORT must be an integer 1-65535, got: ${raw}`);
  }
  return port;
}

function isCI(): boolean {
  const ci = process.env.CI;
  return !!ci && ci !== "false" && ci !== "0" && ci !== "";
}

const port = requirePort();
const baseURL = process.env.DOCS_SITE_BASE_URL ?? `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./tests/playwright",
  fullyParallel: false,
  forbidOnly: isCI(),
  // No retries (CI release-ready program W1b, 2026-11-15): the no-retry-to-green ruling.
  // With `retries: 2` the a11y scan was attempted three times at ~3 min each (runs
  // 43709134600 and 43720285510); the 10-minute step limit in docs-site.yml cancelled
  // gate:behavioral during the second retry, so the first failure's reason was hidden
  // instead of reported.
  retries: 0,
  workers: 1,
  reporter: isCI() ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    browserName: "chromium",
    trace: "retain-on-failure",
  },
  projects: [
    { name: "behavioral", testMatch: /.*\.spec\.ts/ },
    { name: "a11y", testMatch: /.*\.a11y\.ts/ },
    { name: "proven-red", testMatch: /.*\.proven-red\.ts/ },
  ],
  webServer: {
    command: `npx serve build -l ${port}`,
    url: baseURL,
    reuseExistingServer: !isCI(),
    timeout: 120_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
