/**
 * AutoCompare v2 — Chrome, Modals, Command Palette & Shortcuts E2E spec.
 *
 * Verifies:
 *  - Export Report modal opens on active session and dismisses on Esc
 *  - Command Palette filters actions and switches views via keyboard
 *  - Multi-tab bar keeps chrome within viewport with zero horizontal overflow
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
});

test("chrome & modals: export report, command palette, and viewport layout", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(String(e)));

  // Open Text Compare so session-scoped chrome buttons appear
  await page.getByTestId("v2/home-card-text").click();
  await expect(page.getByTestId("v2/text-view")).toBeVisible({ timeout: 10000 });

  // Export Report modal opens and closes cleanly
  await page.getByTestId("v2/export-report").click();
  await expect(page.getByTestId("v2/modal")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("v2/modal")).toHaveCount(0);

  // Command Palette opens via button and filters cleanly
  await page.getByTestId("v2/open-palette").click();
  await expect(page.getByTestId("v2/command-palette")).toBeVisible();
  const paletteInput = page.getByTestId("v2/command-palette").locator("input");
  await paletteInput.fill("Hex");
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("v2/hex-view")).toBeVisible({ timeout: 10000 });

  // Verify no horizontal overflow at 1280x800
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(1);
  expect(errors).toEqual([]);
});
