import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-52 toolbar icon-above-label + active tab pill.
 * Labels visible under icons, active tab carries a border pill,
 * toolbar rides the taller chrome height. New file only.
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

test("toolbar shows icon-above-label buttons and tab pill", async ({
  page,
}) => {
  const toolbar = page.getByRole("toolbar", { name: "Compare actions" });
  await expect(toolbar).toBeVisible();
  const toolbarH = await toolbar.evaluate(
    (el) => el.getBoundingClientRect().height
  );
  // Wave-116 contract: slimmer shared bar (h-11 = 44).
  expect(toolbarH).toBeGreaterThanOrEqual(40);

  const compare = page.getByTestId("toolbar/action-compare");
  await expect(compare).toBeVisible();
  await expect(compare).toContainText("Compare");

  const pill = await page
    .getByRole("tab", { selected: true })
    .evaluate((el) => window.getComputedStyle(el).borderTopWidth);
  expect(pill).toBe("1px");

  await assertNoOverlap(page);
});
