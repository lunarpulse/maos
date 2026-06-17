import { expect, test } from "@playwright/test";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

export function expectedVersionDropdownMode(versionCount: number): "absent" | "present" {
  return versionCount <= 1 ? "absent" : "present";
}

function startFixtureServer(): Promise<{ server: http.Server; port: number; stop: () => void }> {
  const fixtureDir = path.join(__dirname, "fixtures", "version-dropdown-multi");
  const server = http.createServer((req, res) => {
    const filePath = path.join(fixtureDir, req.url === "/" ? "index.html" : req.url ?? "index.html");
    fs.readFile(filePath, (err, data) => {
      if (err) {
        res.writeHead(404);
        res.end("not found");
        return;
      }
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(data);
    });
  });
  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address();
      const port = typeof addr === "object" && addr ? addr.port : 0;
      resolve({
        server,
        port,
        stop: () => server.close(),
      });
    });
  });
}

test.describe("gate:version-dropdown", () => {
  test("pre-1.0 single-version ABI pages resolve and render no version dropdown", async ({ page }) => {
    await page.goto("/abi/v1/lifecycle", { waitUntil: "domcontentloaded" });
    await expect(page.locator("main")).toBeVisible();
    await expect(page.locator('[class*="docsVersionDropdown"], [aria-label*="version" i], [aria-label*="Version"]')).toHaveCount(0);
  });

  test(">=2 fixture branch proves dropdown presence and version switching", async ({ page }) => {
    const { port, stop } = await startFixtureServer();
    try {
      await page.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
      const dropdown = page.locator(".dropdown.docsVersionDropdown");
      await expect(dropdown).toBeVisible();
      await dropdown.locator("button").click();
      const options = dropdown.locator(".dropdown__menu a, [role='menu'] a");
      await expect(options).toHaveCount(2);
      const labels = await options.allInnerTexts();
      expect(labels).toEqual(expect.arrayContaining(["v1", "v2"]));
      await options.filter({ hasText: "v2" }).click();
      await expect(page.locator("main")).toContainText("ABI v2 placeholder");
    } finally {
      stop();
    }
  });
});
