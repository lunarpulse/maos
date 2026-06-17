import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { koPath, loadManifest, manifestRoutePaths } from "./manifest";


function fingerprint(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

test.describe("gate:a11y", () => {
  test.setTimeout(180_000);

  test("axe-core reports zero automated WCAG A/AA violations over distinct rendered pages", async ({ page }) => {
    const manifest = loadManifest();
    const routes = manifestRoutePaths(manifest);
    const scanned = new Set<string>();
    let koRendered = 0;
    let koTranslated = 0;
    let reachedNetwork = 0;
    const missingMain: string[] = [];
    if (manifest.expected_distinct_pages === undefined) {
      throw new Error("route-manifest.json is missing expected_distinct_pages baseline");
    }
    const distinctExpected = manifest.expected_distinct_pages;

    for (const route of routes) {
      for (const localePath of [route, koPath(route)]) {
        const response = await page.goto(localePath, { waitUntil: "domcontentloaded" });
        expect(response?.status(), localePath).toBeLessThan(400);
        reachedNetwork++;

        const mainCount = await page.locator("main").count();
        if (mainCount === 0) {
          missingMain.push(localePath);
          continue;
        }
        const mainText = await page.locator("main").innerText();
        const key = fingerprint(mainText);
        if (localePath.startsWith("/ko/")) {
          koRendered++;
          if ((await page.locator("html").getAttribute("lang")) === "ko") koTranslated++;
        }
        if (scanned.has(key)) continue;
        scanned.add(key);
        const axePage = page as unknown as ConstructorParameters<typeof AxeBuilder>[0]["page"];
        const results = await new AxeBuilder({ page: axePage })
          .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
          .analyze();
        expect(results.violations, localePath).toEqual([]);
      }
    }

    expect(reachedNetwork).toBeGreaterThan(0);
    expect(scanned.size).toBeGreaterThan(0);
    expect(
      missingMain,
      `pages missing \u003cmain\u003e landmark: ${missingMain.join(", ")}`
    ).toEqual([]);
    expect(
      Math.abs(scanned.size - distinctExpected) <= 1,
      `distinct_pages_scanned=${scanned.size} distinct_pages_expected=${distinctExpected}`
    ).toBe(true);
  });
});
