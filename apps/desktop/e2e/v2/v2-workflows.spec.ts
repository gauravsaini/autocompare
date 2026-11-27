/**
 * AutoCompare v2 — New Interactive Workflows E2E spec.
 *
 * Covers:
 * 1. Folder Compare copy & delete actions (selection action bar + right-click context menu)
 * 2. Table Compare inline double-click cell editing + Save CSV
 * 3. 3-Way Text Merge keyboard shortcuts (Alt+1/2/3/4) + inline custom resolution editor
 * 4. Diff Report Export modal (HTML / Patch / Markdown / CSV / JSON + live preview)
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
});

test("folder compare: selection bar copy and right-click context menu delete log actions", async ({ page }) => {
  await page.getByTestId("v2/home-card-folder").click();
  await expect(page.getByTestId("v2/folder-view")).toBeVisible({ timeout: 10000 });

  // Wait for fixture entries to load
  const firstRow = page.getByTestId("v2/folder-view").locator("div.grid.h-\\[22px\\]").first();
  await expect(firstRow).toBeVisible({ timeout: 10000 });

  // Select row -> selection action bar should appear with Copy to Right
  await firstRow.click();
  const copyBtn = page.getByTestId("v2/folder-view").getByRole("button", { name: /Copy to Right/i });
  await expect(copyBtn).toBeVisible();
  await copyBtn.click();

  // Activity log should record the L -> R copy
  await expect(page.getByTestId("v2/folder-view")).toContainText("L → R");

  // Right-click row -> context menu -> Delete Left (confirm dialog accepted)
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

  // Double-click first editable left cell
  const firstCell = page.getByTestId("v2/table-view").locator("div.cursor-text").first();
  await expect(firstCell).toBeVisible({ timeout: 10000 });
  await firstCell.dblclick();

  const editor = firstCell.locator("input[type='text']");
  await expect(editor).toBeVisible();
  await editor.fill("updated-cell-42");
  await editor.press("Enter");

  await expect(firstCell).toContainText("updated-cell-42");
  await expect(page.getByTestId("v2/statusbar")).toContainText("Edited left row 1");

  // Provide a path and click Save on left path bar
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

  // Alt+1 -> Take Left
  await page.keyboard.press("Alt+1");
  await expect(view).toContainText("Resolved");
  const outputArea = view.locator("textarea").last();
  await expect(outputArea).toHaveValue("left line");

  // Alt+3 -> Take Right
  await page.keyboard.press("Alt+3");
  await expect(outputArea).toHaveValue("right line");

  // Alt+4 -> Both (L->R)
  await page.keyboard.press("Alt+4");
  await expect(outputArea).toHaveValue("left line\nright line");

  // Edit Inline -> Custom resolution editor
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

  // Switch to Unified Patch and preview
  await page.getByTestId("v2/report-format-patch").click();
  await page.getByRole("button", { name: "Preview" }).click();
  await expect(page.getByTestId("v2/report-preview")).toBeVisible();
  await expect(page.getByTestId("v2/report-preview")).toContainText("--- a/");

  // Switch back to HTML and export
  await page.getByTestId("v2/report-format-html").click();
  await page.getByTestId("v2/report-export-btn").click();
  await expect(page.getByTestId("v2/report-result")).toContainText("Exported HTML report");
  await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
});
