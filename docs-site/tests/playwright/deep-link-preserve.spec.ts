import { expect, test } from "@playwright/test";

const cases = [
  { route: "/ko/abi/v1/lifecycle", anchor: "maos-spirit-abi-lifecycle-spirit" },
  { route: "/ko/abi/v1/ctx", anchor: "maos-spirit-abi-ctx-ctx" },
  { route: "/ko/abi/v1/identity", anchor: "maos-spirit-abi-identity-framekind" },
];

test.describe("gate:deep-link-preserve", () => {
  for (const { route, anchor } of cases) {
    test(`locale-invariant explicit ABI anchor exists in rendered ko DOM: ${route}#${anchor}`, async ({ page }) => {
      await page.goto(`${route}#${anchor}`, { waitUntil: "domcontentloaded" });
      await expect(page.locator(`#${anchor}`)).toBeVisible();
      expect(new URL(page.url()).hash).toBe(`#${anchor}`);
    });
  }
});
