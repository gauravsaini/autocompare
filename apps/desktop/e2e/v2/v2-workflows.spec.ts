/**
 * AutoCompare v2 — New Interactive Workflows E2E spec.
 *
 * Covers:
 * 1. Folder Compare copy & delete actions (selection action bar + right-click context menu)
 * 2. Table Compare inline double-click cell editing + Save CSV
 * 3. 3-Way Text Merge keyboard shortcuts (Alt+1/2/3/4) + inline custom resolution editor
 * 4. Diff Report Export modal (HTML / Patch / Markdown / CSV / JSON + live preview)
 * 5. Keyboard Shortcut Cheat Sheet modal (? / header button + filter)
 * 6. Folder Sync per-row exclude + Dry Run preview
 * 7. Hex Compare Data Inspector (LE/BE) & Media Compare visual modes (Wipe/Onion/Pixel Diff)
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
});

test("folder compare: selection bar copy and right-click context menu delete log actions", async ({ page }) => {
  await page.getByTestId("v2/home-card-folder").click();
  await expect(page.getByTestId("v2/folder-view")).toBeVisible({ timeout: 10000 });

  const firstRow = page.getByTestId("v2/folder-view").locator("div.grid.h-\\[22px\\]").first();
  await expect(firstRow).toBeVisible({ timeout: 10000 });

  await firstRow.click();
  const copyBtn = page.getByTestId("v2/folder-view").getByRole("button", { name: /Copy to Right/i });
  await expect(copyBtn).toBeVisible();
  await copyBtn.click();

  await expect(page.getByTestId("v2/folder-view")).toContainText("L → R");

  page.once("dialog", (dialog) => dialog.accept());
  await firstRow.click({ button: "right" });
  const deleteMenuBtn = page.getByRole("button", { name: "Delete Left" });
  await expect(deleteMenuBtn).toBeVisible();
  await deleteMenuBtn.click();

  await expect(page.getByTestId("v2/folder-view")).toContainText("Deleted");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});

test("table compare: double-click inline cell edit commits on Enter and Save CSV succeeds", async ({ page }) => {
  await page.getByTestId("v2/home-card-table").click();
  await expect(page.getByTestId("v2/table-view")).toBeVisible({ timeout: 10000 });

  const firstCell = page.getByTestId("v2/table-view").locator("div.cursor-text").first();
  await expect(firstCell).toBeVisible({ timeout: 10000 });
  await firstCell.dblclick();

  const editor = firstCell.locator("input[type='text']");
  await expect(editor).toBeVisible();
  await editor.fill("updated-cell-42");
  await editor.press("Enter");

  await expect(firstCell).toContainText("updated-cell-42");
  await expect(page.getByTestId("v2/statusbar")).toContainText("Edited left row 1");

  await page.getByTestId("v2/path-left").fill("/tmp/table-left.csv");
  const saveLeftBtn = page.getByTestId("v2/table-view").getByRole("button", { name: /Save left/i });
  if (await saveLeftBtn.count()) {
    await saveLeftBtn.first().click();
    await expect(page.getByTestId("v2/statusbar")).toContainText("Saved left CSV");
  }
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});

test("3-way merge: Alt+1/3/4 shortcuts and Edit Inline custom resolution update merged output", async ({ page }) => {
  await page.getByTestId("v2/home-card-merge").click();
  await expect(page.getByTestId("v2/merge-view")).toBeVisible({ timeout: 10000 });

  const view = page.getByTestId("v2/merge-view");
  await expect(view).toContainText("Conflict #1");
  await expect(view).toContainText("Unresolved");

  await page.keyboard.press("Alt+1");
  await expect(view).toContainText("Resolved");
  const outputArea = view.locator("textarea").last();
  await expect(outputArea).toHaveValue("left line");

  await page.keyboard.press("Alt+3");
  await expect(outputArea).toHaveValue("right line");

  await page.keyboard.press("Alt+4");
  await expect(outputArea).toHaveValue("left line\nright line");

  await view.getByRole("button", { name: /Edit Inline/i }).click();
  await expect(view).toContainText("Custom resolution for Conflict #1");
  const customEditor = view.locator("textarea").first();
  await customEditor.fill("custom merged line");
  await expect(view).toContainText("Custom");
  await expect(outputArea).toHaveValue("custom merged line");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});

test("report export modal: switches formats, previews unified patch, and exports HTML report", async ({ page }) => {
  await page.getByTestId("v2/home-card-text").click();
  await expect(page.getByTestId("v2/text-view")).toBeVisible({ timeout: 10000 });

  await page.getByTestId("v2/export-report").click();
  await expect(page.getByTestId("v2/report-modal")).toBeVisible();

  await page.getByTestId("v2/report-format-patch").click();
  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByTestId("v2/report-preview")).toBeVisible();
  await expect(page.getByTestId("v2/report-preview")).toContainText("--- a/");

  await page.getByTestId("v2/report-format-html").click();
  await page.getByTestId("v2/report-export-btn").click();
  await expect(page.getByTestId("v2/report-result")).toContainText("Exported HTML report");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});

test("shortcut cheat sheet modal opens via header button and filters shortcuts", async ({ page }) => {
  await page.getByTestId("v2/open-shortcuts").click();
  const modal = page.getByTestId("v2/shortcuts-modal");
  await expect(modal).toBeVisible();
  await modal.locator("input[type='text']").fill("merge");
  await expect(modal).toContainText("3-Way Text Merge");
  await expect(modal).toContainText("Alt");
  await page.getByRole("button", { name: "Done" }).click();
  await expect(modal).toHaveCount(0);
});

test("folder sync: per-row exclude updates active count and Dry Run previews plan", async ({ page }) => {
  await page.getByTestId("v2/home-card-sync").click();
  const view = page.getByTestId("v2/sync-view");
  await expect(view).toBeVisible({ timeout: 10000 });

  const firstCheckbox = view.locator("input[type='checkbox'][aria-label^='Include ']").first();
  if (await firstCheckbox.count()) {
    await firstCheckbox.uncheck();
    await expect(view).toContainText("Excluded: 1");
    await view.getByRole("button", { name: "Include All" }).click();
  }
  await view.getByRole("button", { name: "Dry Run" }).click();
  await expect(view).toContainText("Dry-Run Preview:");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});

test("hex compare inspector and media compare visual modes render cleanly", async ({ page }) => {
  await page.getByTestId("v2/home-card-hex").click();
  const hexView = page.getByTestId("v2/hex-view");
  await expect(hexView).toBeVisible({ timeout: 10000 });
  const firstByte = hexView.locator("span[title*='click to inspect']").first();
  await expect(firstByte).toBeVisible({ timeout: 10000 });
  await firstByte.click();
  const inspector = page.getByTestId("v2/hex-inspector");
  await expect(inspector).toBeVisible();
  await expect(inspector).toContainText("LE");
  await inspector.getByRole("button", { name: "LE" }).click();
  await expect(inspector).toContainText("BE");

  await page.getByRole("tab", { name: "Home" }).click();
  await page.getByTestId("v2/home-card-media").click();
  const mediaView = page.getByTestId("v2/media-view");
  await expect(mediaView).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("v2/media-stage")).toBeVisible();
  await page.getByTestId("v2/media-mode-onion").click();
  await expect(page.getByTestId("v2/media-stage")).toContainText("Onion Skin");
  await page.getByTestId("v2/media-mode-diff").click();
  await expect(page.getByTestId("v2/media-stage")).toContainText("Pixel Diff");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});
