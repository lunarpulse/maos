import { expect, test } from "@playwright/test";

test.describe("gate:fallback", () => {
  test("untranslated ko ABI route serves English content with English page lang and Korean banner", async ({ page }) => {
    await page.goto("/ko/abi/v1/lifecycle", { waitUntil: "domcontentloaded" });

    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await expect(page.getByText("The Spirit lifecycle trait.")).toBeVisible();

    const banner = page.locator('[data-maos-fallback-banner="ko"]');
    await expect(banner).toBeVisible();
    await expect(banner).toHaveAttribute("lang", "ko");
    await expect(banner).toContainText("한국어 번역이 아직 없어 영어 원문을 표시합니다");
  });

  test("translated ko docs keep Korean page lang", async ({ page }) => {
    await page.goto("/ko/write-a-spirit", { waitUntil: "domcontentloaded" });
    await expect(page.locator("html")).toHaveAttribute("lang", "ko");
  });
});
