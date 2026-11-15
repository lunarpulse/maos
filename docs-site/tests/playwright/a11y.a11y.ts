import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { koPath, loadManifest, manifestRoutePaths } from "./manifest";


function fingerprint(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

// Budget (CI release-ready program W1b, 2026-11-15). This is ONE test that visits
// 123 manifest routes x 2 locales (246 pages) and runs axe on the ~176 distinct ones. Hosted
// runs of the same content took 2.1 min (43708454791) and 2.5 min (43708448894). In the
// failed runs 43709134600 and 43720285510 the first scan reached the LAST route
// (/ko/errors/ETeamIdInvalid) about 188 s and 182 s after its first request, i.e. at or just
// past the old 180 s ceiling; the first attempt and retry #1 were each reported failed at
// 3.2 min, and the 10-minute step limit cancelled the job during retry #2, before any error
// report. A per-test timeout is consistent with that evidence but is not conclusively
// proven (the failure message was never printed). It is not a hang: the scan was making
// steady progress through the last pages. The budget is now 360 s (about 2x the slowest
// observed scan), and the per-page progress lines below record where the time goes, so the
// next overrun leaves its own evidence in the log.
const SCAN_BUDGET_MS = 360_000;
const PROGRESS_EVERY_PAGES = 20;

test.describe("gate:a11y", () => {
  test.setTimeout(SCAN_BUDGET_MS);

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

    // Diagnostic only (no assertion depends on it): where the scan spends its time, printed
    // as the scan runs so a timeout still leaves the evidence in the log.
    const totalPages = routes.length * 2;
    const startedAt = Date.now();
    let gotoMs = 0;
    let axeMs = 0;
    const timings: { path: string; ms: number }[] = [];
    const report = (label: string, current: string) => {
      const slowest = [...timings]
        .sort((a, b) => b.ms - a.ms)
        .slice(0, 3)
        .map((t) => `${t.path}=${t.ms}ms`)
        .join(" ");
      console.log(
        `a11y-scan ${label}: pages=${reachedNetwork}/${totalPages} distinct=${scanned.size} ` +
          `elapsed=${Math.round((Date.now() - startedAt) / 1000)}s/${SCAN_BUDGET_MS / 1000}s ` +
          `goto_ms=${gotoMs} axe_ms=${axeMs} last=${current} slowest=[${slowest}]`
      );
    };

    for (const route of routes) {
      for (const localePath of [route, koPath(route)]) {
        const gotoStart = Date.now();
        const response = await page.goto(localePath, { waitUntil: "domcontentloaded" });
        gotoMs += Date.now() - gotoStart;
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
        if (!scanned.has(key)) {
          scanned.add(key);
          const axeStart = Date.now();
          const axePage = page as unknown as ConstructorParameters<typeof AxeBuilder>[0]["page"];
          const results = await new AxeBuilder({ page: axePage })
            .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
            .analyze();
          const elapsed = Date.now() - axeStart;
          axeMs += elapsed;
          timings.push({ path: localePath, ms: elapsed });
          expect(results.violations, localePath).toEqual([]);
        }
        if (reachedNetwork % PROGRESS_EVERY_PAGES === 0) report("progress", localePath);
      }
    }
    report("done", "-");

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
