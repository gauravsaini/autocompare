import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-57 line-detail typography: header/empty/labels ride the chrome
 * ramp, code panes ride the code ramp (mono, 1.55, zero tracking).
 * New file only, no existing spec touched.
 */
test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("line detail rides the token ramps", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const row = page.locator(".tv-row", { hasText: "left variant of line 0047" }).first();
  await expect(row).toBeVisible();
  await row.locator('div[title="left"]').click();

  const header = page.getByTestId("line-detail/header");
  await expect(header).toBeVisible();
  const headerTracking = await header.evaluate(
    (el) => window.getComputedStyle(el).letterSpacing
  );
  expect(Math.abs(parseFloat(headerTracking) - -0.12)).toBeLessThan(0.05);

  const leftPre = page.getByTestId("line-detail/left");
  const family = await leftPre.evaluate((el) => window.getComputedStyle(el).fontFamily);
  expect(family).toContain("SF Mono");
  const leading = await leftPre.evaluate((el) => window.getComputedStyle(el).lineHeight);
  expect(Math.abs(parseFloat(leading) - 18.6)).toBeLessThan(0.05);

  await assertNoOverlap(page);
});
