import { expect, test } from "@playwright/test";
import {
  buildPageRoutes,
  koPath,
  jaPath,
  zhPath,
  loadManifest,
  manifestRoutePaths,
  type RouteManifest,
} from "./manifest";

function normalizeRoute(route: string): string {
  return route.replace(/\/$/, "") || "/";
}

function expectedPageRoutes(manifest: RouteManifest): Set<string> {
  const enRoutes = manifestRoutePaths(manifest);
  const expected = new Set<string>();
  for (const route of enRoutes) {
    expected.add(normalizeRoute(route));
    expected.add(normalizeRoute(koPath(route)));
    expected.add(normalizeRoute(jaPath(route)));
    expected.add(normalizeRoute(zhPath(route)));
  }
  for (const redirect of manifest.redirects ?? []) {
    expected.add(normalizeRoute(redirect.from));
    expected.add(normalizeRoute(koPath(redirect.from)));
    expected.add(normalizeRoute(jaPath(redirect.from)));
    expected.add(normalizeRoute(zhPath(redirect.from)));
  }
  return expected;
}

const manifest = loadManifest();
const expectedRoutes = expectedPageRoutes(manifest);
const actualRoutes = new Set(buildPageRoutes().map(normalizeRoute));

test.describe("gate:links", () => {
  for (const route of manifestRoutePaths(manifest)) {
    test(`manifest route resolves: ${route}`, async ({ page }) => {
      const response = await page.goto(route, { waitUntil: "domcontentloaded" });
      expect(response?.status(), route).toBeLessThan(400);
      await expect(page.locator("main")).toBeVisible();
    });
  }

  for (const redirect of manifest.redirects ?? []) {
    test(`redirect target resolves: ${redirect.from} -> ${redirect.to}`, async ({ page }) => {
      const target = await page.goto(redirect.to, { waitUntil: "domcontentloaded" });
      expect(target?.status(), redirect.to).toBeLessThan(400);

      await page.goto(redirect.from, { waitUntil: "domcontentloaded" });
      await expect
        .poll(() => normalizeRoute(new URL(page.url()).pathname))
        .toBe(normalizeRoute(redirect.to));
    });
  }

  test("no manifest route is missing a built page", () => {
    const missing = [...expectedRoutes].filter((route) => !actualRoutes.has(route));
    expect(missing, `missing built pages: ${missing.join(", ")}`).toEqual([]);
  });

  test("no built page is orphaned from the manifest", () => {
    const orphaned = [...actualRoutes].filter((route) => !expectedRoutes.has(route));
    expect(orphaned, `orphan pages: ${orphaned.join(", ")}`).toEqual([]);
  });
});
