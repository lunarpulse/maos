import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

// The color-mode toggle lives in the navbar; the mobile nav hamburger
// (aria-label "Toggle navigation bar") does NOT contain "mode", so this is unambiguous.
const toggle = (page: Page) => page.locator('.navbar button[aria-label*="mode" i]').first();

const dataTheme = (page: Page) =>
  page.evaluate(() => document.documentElement.getAttribute("data-theme"));

// --ifm-background-color flips between light (#fff) and dark (#1b1b1d); a robust
// signal that the dark theme actually applied, not just that the attribute changed.
const backgroundVar = (page: Page) =>
  page.evaluate(() =>
    getComputedStyle(document.documentElement)
      .getPropertyValue("--ifm-background-color")
      .trim()
  );

test.describe("gate:theme", () => {
  test("one click flips light<->dark and applies the dark theme", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.waitForSelector('html[data-has-hydrated="true"]');

    // respectPrefersColorScheme:false => deterministic default light, no "system" state.
    expect(await dataTheme(page)).not.toBe("dark");
    const lightBg = await backgroundVar(page);

    const btn = toggle(page);
    await expect(btn).toHaveCount(1);

    // ONE click must reach dark (the system->light->dark papercut is gone).
    await btn.click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

    // Dark mode must change a visible style, not just the attribute.
    const darkBg = await backgroundVar(page);
    expect(darkBg).not.toBe(lightBg);

    // One more click returns to light.
    await btn.click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  });
});
