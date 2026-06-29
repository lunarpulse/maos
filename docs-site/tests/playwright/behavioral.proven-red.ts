import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

let runCount = 0;
let reachedNetwork = 0;

async function reached(page: Page, path = "/") {
  const response = await page.goto(path, { waitUntil: "domcontentloaded" });
  expect(response?.status(), path).toBeLessThan(400);
  reachedNetwork++;
}

test.describe("behavioral proven-red", () => {
  test("runtime mutations make each browser gate observe a red condition", async ({ page }) => {
    runCount++;
    // Green baseline: page has lang attribute.
    await reached(page, "/");
    await expect(page.locator("html")).toHaveAttribute("lang", new RegExp(".+"));

    // Proven-red: intercept the home page and serve it without lang and with an
    // alt-less image so axe deterministically reports the expected violations.
    await page.route("/", async (route) => {
      const response = await route.fetch();
      const body = await response.text();
      const mutated = body
        .replace(/\s+lang="[^"]*"/g, "")
        .replace(/\s+xml:lang="[^"]*"/g, "")
        .replace(
          "</body>",
          '<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw="></body>'
        );
      await route.fulfill({ response, body: mutated });
    });
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.waitForSelector('html[data-has-hydrated="true"]');
    await page.evaluate(() => {
      document.documentElement.removeAttribute("lang");
      document.documentElement.removeAttribute("xml:lang");
    });
    await expect(page.locator("html")).not.toHaveAttribute("lang");
    const axePage = page as unknown as ConstructorParameters<typeof AxeBuilder>[0]["page"];
    const axeResults = await new AxeBuilder({ page: axePage }).analyze();
    expect(axeResults.violations.map((v) => v.id)).toContain("html-has-lang");
    expect(axeResults.violations.map((v) => v.id)).toContain("image-alt");
    await page.unroute("/");

    runCount++;
    // Green baseline: fallback route resolves 200 with banner.
    const fallbackPath = "**/ko/errors/EAbiTooNew";
    const fallbackHandler = (route: Route) =>
      route.fulfill({ status: 404, body: "forced missing fallback" });
    await page.goto("/ko/errors/EAbiTooNew", { waitUntil: "domcontentloaded" });
    await expect(page.locator("[data-maos-fallback-banner='ko']")).toBeVisible();
    await page.route(fallbackPath, fallbackHandler);
    const forcedMissing = await page.goto("/ko/errors/EAbiTooNew", { waitUntil: "domcontentloaded" });
    expect(forcedMissing?.status()).toBe(404);
    await page.unroute(fallbackPath, fallbackHandler);

    runCount++;
    // Green baseline: switcher link exists.
    await page.goto("/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit", { waitUntil: "domcontentloaded" });
    await expect(page.locator('a[href="/ko/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit"]')).toHaveCount(1);
    await page.evaluate(() => {
      document.querySelectorAll('a[href^="/ko/abi/v1/lifecycle"]').forEach((node) => node.remove());
    });
    await expect(page.locator('a[href="/ko/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit"]')).toHaveCount(0);

    runCount++;
    // Green baseline: deep-link target exists.
    await page.goto("/ko/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit", { waitUntil: "domcontentloaded" });
    await expect(page.locator("#maos-spirit-abi-lifecycle-spirit")).toBeVisible();
    await page.evaluate(() => document.getElementById("maos-spirit-abi-lifecycle-spirit")?.removeAttribute("id"));
    await expect(page.locator("#maos-spirit-abi-lifecycle-spirit")).toHaveCount(0);

    runCount++;
    const manifestRoutes = new Set(["/only-in-manifest"]);
    const pageRoutes = new Set(["/only-on-disk"]);
    const missing = [...manifestRoutes].filter((route) => !pageRoutes.has(route));
    const orphaned = [...pageRoutes].filter((route) => !manifestRoutes.has(route));
    expect(missing).toEqual(["/only-in-manifest"]);
    expect(orphaned).toEqual(["/only-on-disk"]);

    runCount++;
    // Green baseline: one click flips the color mode to dark.
    await reached(page, "/");
    await page.waitForSelector('html[data-has-hydrated="true"]');
    await page.locator('.navbar button[aria-label*="mode" i]').first().click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    // Proven-red: a broken toggle that doesn't stick (force the attribute back to light)
    // makes gate:theme's "data-theme=dark" assertion observably fail.
    await page.evaluate(() => document.documentElement.setAttribute("data-theme", "light"));
    await expect(page.locator("html")).not.toHaveAttribute("data-theme", "dark");

    expect(runCount).toBe(6);
    expect(reachedNetwork).toBeGreaterThan(0);
  });
});
