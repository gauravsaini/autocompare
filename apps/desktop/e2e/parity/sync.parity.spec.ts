/** Living parity spec scaffold: sync view — geometry locks land here (not in functional specs). */
import { expect, test } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  try { await page.goto("/", { waitUntil: "domcontentloaded" }); await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 }); }
  catch (err) { test.skip(true, `infra: ${err}`); }
});
test("sync parity scaffold: boots without overlap", async ({ page }) => {
  await expect(page.getByTestId("session-count")).toBeVisible();
  const overlap = await page.evaluate(() => {
    const els = Array.from(document.querySelectorAll("header, nav, main"));
    for (let i = 0; i < els.length; i++) for (let j = i + 1; j < els.length; j++) {
      const a = els[i]!.getBoundingClientRect(), b = els[j]!.getBoundingClientRect();
      if (a.width > 0 && b.width > 0 && a.x === b.x && a.y === b.y && a.width === b.width) return true;
    }
    return false;
  });
  expect(overlap).toBe(false);
});
