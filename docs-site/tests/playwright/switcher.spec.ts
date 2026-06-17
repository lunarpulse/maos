import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function expectLocaleHref(page: Page, href: string) {
  const links = page.locator(`a[href="${href}"]`);
  await expect(links).toHaveCount(1);
}

test.describe("gate:switcher", () => {
  test("preserves anchor fragments", async ({ page }) => {
    await page.goto("/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit", { waitUntil: "domcontentloaded" });
    await expectLocaleHref(page, "/ko/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit");
  });

  test("preserves fallback round-trip URL", async ({ page }) => {
    await page.goto("/ko/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit", { waitUntil: "domcontentloaded" });
    await expectLocaleHref(page, "/abi/v1/lifecycle#maos-spirit-abi-lifecycle-spirit");
  });

  test("preserves version segment with locale prefix ordering", async ({ page }) => {
    await page.goto("/abi/v1/ctx", { waitUntil: "domcontentloaded" });
    await expectLocaleHref(page, "/ko/abi/v1/ctx");
  });

  test("normalizes trailing slash and carries query", async ({ page }) => {
    await page.goto("/abi/v1/?view=auditor", { waitUntil: "domcontentloaded" });
    await expectLocaleHref(page, "/ko/abi/v1/?view=auditor");
  });
});
