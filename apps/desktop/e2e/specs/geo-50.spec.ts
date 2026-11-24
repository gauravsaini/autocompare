import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-50 exquisite geometry: toolbar density, PathBar alignment,
 * Welcome vertical rhythm, chrome heights on the 4px spacing scale.
 * Geometry only, colors and fonts frozen.
 */
test("wave-50 chrome geometry + no overlap", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("tabbar")).toBeVisible();
  await expect(page.getByRole("toolbar", { name: "Compare actions" })).toBeVisible();
  await expect(page.getByTestId("pathbar")).toBeVisible();
  await expect(page.getByTestId("statusbar")).toBeVisible();

  const toolbarH = await page.getByRole("toolbar", { name: "Compare actions" }).evaluate((el) => el.getBoundingClientRect().height);
  // Wave-116 contract: slimmer shared bar (h-11 = 44).
  expect(toolbarH).toBeGreaterThanOrEqual(40);
  expect(toolbarH).toBeLessThanOrEqual(48);

  const tabbarH = await page.getByTestId("tabbar").evaluate((el) => el.getBoundingClientRect().height);
  expect(tabbarH).toBeGreaterThanOrEqual(36);
  expect(tabbarH).toBeLessThanOrEqual(44);

  const pathbarH = await page.getByTestId("pathbar").evaluate((el) => el.getBoundingClientRect().height);
  // Wave-116 contract: compact single-row pathbar (h-8 = 32).
  expect(pathbarH).toBeGreaterThanOrEqual(28);
  expect(pathbarH).toBeLessThanOrEqual(36);

  const left = await page.getByTestId("pathbar/left").evaluate((el) => el.getBoundingClientRect());
  const right = await page.getByTestId("pathbar/right").evaluate((el) => el.getBoundingClientRect());
  expect(Math.abs(left.width - right.width)).toBeLessThanOrEqual(8);

  await assertNoOverlap(page);
});
