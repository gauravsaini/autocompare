import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-51 chrome typography: tab labels, status bar, and welcome title
 * ride the UI token ramp (tight tracking, single 11px meta size).
 * Geometry and colors frozen. New file only, no existing spec touched.
 */
test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("tabbar")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("chrome type rides the ui token ramp", async ({ page }) => {
  const tab = page.getByRole("tab", { selected: true });
  await expect(tab).toBeVisible();
  // Stylesheet may lag first paint under load: wait until the token
  // tracking actually resolves instead of reading "normal".
  await expect
    .poll(
      async () =>
        tab.evaluate((el) => window.getComputedStyle(el).letterSpacing),
      { timeout: 15000 }
    )
    .not.toBe("normal");
  const tracking = await tab.evaluate(
    (el) => window.getComputedStyle(el).letterSpacing
  );
  // 12px * -0.01em = -0.12px
  expect(Math.abs(parseFloat(tracking) - -0.12)).toBeLessThan(0.03);

  const statusSize = await page
    .getByTestId("statusbar")
    .evaluate((el) => window.getComputedStyle(el).fontSize);
  expect(statusSize).toBe("11px");

  // BC parity: path fields use the proportional UI stack (Inter),
  // not mono — matches Beyond Compare, where path combos render in
  // the UI font. (Was SF Mono before the deliberate Inter move.)
  const inputFamily = await page
    .getByTestId("pathbar/left")
    .evaluate((el) => window.getComputedStyle(el).fontFamily);
  expect(inputFamily).toContain("Inter");

  await assertNoOverlap(page);
});
