/**
 * AutoCompare v2 — Text Compare living E2E spec.
 *
 * Verifies:
 *  - Demo diff rows render immediately with zero error banner
 *  - Dual path bar inputs (v2/path-left, v2/path-right) and toolbar filters work cleanly
 *  - Direct click-to-type notepad editing, double-click word selection (no collapse),
 *    and Enter line split with live re-diff
 *  - Shift+Click multi-line range selection + range merge + Ctrl+Z undo
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

test("text compare: notepad click-to-type, double-click word select, and Enter line split", async ({ page }) => {
  const row0 = page.getByTestId("v2/text-row-0");
  await expect(row0).toBeVisible({ timeout: 10000 });

  // Single-click Left code cell mounts seamless borderless input immediately
  const leftCell = row0.locator("div").first();
  await leftCell.click();
  const editor = row0.locator("input[type='text']").first();
  await expect(editor).toBeVisible();

  // Double-click word inside editor preserves non-empty text selection
  await editor.dblclick();
  const selLen = await editor.evaluate((el: HTMLInputElement) => (el.selectionEnd ?? 0) - (el.selectionStart ?? 0));
  expect(selLen).toBeGreaterThan(0);

  // Type and press Enter to split line live
  await editor.press("End");
  await editor.type(" edited");
  await editor.press("Enter");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
  await expect(page.getByTestId("v2/text-row-1")).toBeVisible();
});

test("text compare: Shift+Click multi-line range merge and Ctrl+Z undo", async ({ page }) => {
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible({ timeout: 10000 });

  // Jump to first diff section via toolbar Next
  await page.getByTestId("v2/toolbar-next").click();
  const detailCopyLeft = page.getByTestId("v2/detail-copy-line-left");
  await expect(detailCopyLeft).toBeVisible();

  // Shift+Click two rows ahead via line-number gutter to form a multi-line range
  const rows = page.locator("[data-testid^='v2/text-row-']");
  const count = await rows.count();
  expect(count).toBeGreaterThan(4);
  await rows.nth(6).locator("span").nth(1).click();
  await rows.nth(9).locator("span").nth(1).click({ modifiers: ["Shift"] });
  await expect(detailCopyLeft).toContainText("Copy 4 Lines");

  // Merge selected range and verify Undo via Ctrl+Z
  await detailCopyLeft.click();
  await expect(page.getByTestId("v2/statusbar")).toContainText("Copied 4 lines");
  await page.keyboard.press("Control+z");
  await expect(page.getByTestId("v2/statusbar")).toContainText("Undid");
});

test("text compare: Rules dialog covers Specs, Format, Importance, Alignment, and Replacements", async ({ page }) => {
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible({ timeout: 10000 });

  // Open Rules dialog from toolbar
  await page.getByTestId("v2/toolbar-rules").click();
  await expect(page.getByTestId("v2/rules-tab-importance")).toHaveAttribute("aria-selected", "true");

  // Specs tab: both file path specs are editable
  await page.getByTestId("v2/rules-tab-specs").click();
  await expect(page.getByTestId("v2/rules-spec-left")).toBeVisible();
  await expect(page.getByTestId("v2/rules-spec-right")).toBeVisible();

  // Format tab: per-side encoding dropdowns visible
  await page.getByTestId("v2/rules-tab-format").click();
  await expect(page.getByTestId("v2/rules-left-encoding")).toBeVisible();
  await expect(page.getByTestId("v2/rules-right-encoding")).toBeVisible();

  // Importance tab: add ISO Timestamps unimportant grammar preset
  await page.getByTestId("v2/rules-tab-importance").click();
  await page.getByTestId("v2/rules-add-preset-0").click();

  // Alignment tab: switch to Unaligned and toggle Never align differences
  await page.getByTestId("v2/rules-tab-alignment").click();
  await page.getByTestId("v2/rules-align-unaligned").check();
  await page.getByTestId("v2/rules-never-align").check();

  // Replacements tab: add a replacement rule
  await page.getByTestId("v2/rules-tab-replacements").click();
  await page.getByTestId("v2/rules-add-replacement").click();

  // Apply and confirm zero errors
  await page.getByTestId("v2/rules-ok").click();
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
  await expect(page.getByTestId("v2/statusbar")).toContainText("Applied session rules");
});

test("text compare: Minor unimportant toggle and F7 manual line alignment", async ({ page }) => {
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible({ timeout: 10000 });

  // Toggle Minor unimportant differences button cleanly
  await page.getByTestId("v2/toolbar-minor").click();
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);

  // Select row 2 and press F7 to start Align With mode
  await page.getByTestId("v2/text-row-2").locator("span").nth(1).click();
  await page.keyboard.press("F7");
  const alignBanner = page.getByTestId("v2/align-banner");
  await expect(alignBanner).toBeVisible();

  // Click a right-side cell on row 4 to pin manual alignment
  await page.getByTestId("v2/text-row-4").locator("div").nth(1).click();
  await expect(alignBanner).toContainText("1 manual alignment");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});

test("text compare: Ctrl+H opens Find & Replace and replaces text cleanly", async ({ page }) => {
  await expect(page.getByTestId("v2/text-row-0")).toBeVisible({ timeout: 10000 });

  await page.keyboard.press("Control+h");
  const findInput = page.locator("input[placeholder*='Find']");
  const replaceInput = page.getByTestId("v2/replace-input");
  await expect(findInput).toBeVisible();
  await expect(replaceInput).toBeVisible();

  await findInput.fill("common");
  await replaceInput.fill("shared");
  await page.getByTestId("v2/replace-all").click();
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
  await expect(page.getByTestId("v2/statusbar")).toContainText("Replaced");
});
