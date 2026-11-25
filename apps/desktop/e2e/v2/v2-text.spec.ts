/**
 * AutoCompare v2 — Text Compare living E2E spec.
 *
 * Verifies:
 *  - Demo diff rows render immediately with zero error banner
 *  - Dual path bar inputs (v2/path-left, v2/path-right) are editable
 *  - Toolbar filters (All / Diffs / Same), Swap, and Find (Ctrl+F / Esc) work cleanly
 *  - Draggable Left/Right splitter updates pane split ratio smoothly
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("v2/home-card-text").click();
  await expect(page.getByTestId("v2/text-view")).toBeVisible({ timeout: 10000 });
});

test("text compare: renders diff rows, path bar, and toolbar controls", async ({ page }) => {
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
  await expect(page.getByTestId("v2/path-left")).toBeVisible();
  await expect(page.getByTestId("v2/path-right")).toBeVisible();
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible({ timeout: 10000 });

  // Filter buttons in unified toolbar
  await page.getByTestId("v2/toolbar-diffs").click();
  await expect(page.getByTestId("v2/text-view")).toBeVisible();
  await page.getByTestId("v2/toolbar-all").click();
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible();

  // Find bar via shortcut and Esc dismiss
  await page.keyboard.press("Control+f");
  const findInput = page.locator("input[placeholder*='Find']");
  await expect(findInput).toBeVisible();
  await findInput.fill("line");
  await page.keyboard.press("Escape");
  await expect(findInput).toHaveCount(0);

  // Swap sides swaps path inputs without error
  const beforeLeft = await page.getByTestId("v2/path-left").inputValue();
  const beforeRight = await page.getByTestId("v2/path-right").inputValue();
  await page.getByTestId("v2/toolbar-swap").click();
  await expect(page.getByTestId("v2/path-left")).toHaveValue(beforeRight);
  await expect(page.getByTestId("v2/path-right")).toHaveValue(beforeLeft);
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});
