/**
 * Wave-66U-b cell edit-mode proof, backend-applied form (wave-66V).
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-boot demo text diff
 * (real engine rows) and proves in-place editing end to end:
 *  - double-click (or Enter on a focused cell) swaps a non-null cell
 *    for an input prefilled with the cell text;
 *  - Enter applies through `text_apply_edit` (mock serves the patched
 *    row on refetch), the override clears, `applied-note` counts it;
 *  - Esc cancels with zero change and no notes;
 *  - orphan (null) cells are a no-op at the UI level;
 *  - reload clears the mock overlay (module state, never storage).
 * New file only, no existing spec touched. Skip policy matches the
 * other specs.
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function firstChangedLeftCell(page: import("@playwright/test").Page) {
  const row = page.locator('.tv-row[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  return { row, cell: row.locator(".tv-cell").first() };
}

test("dblclick opens a prefilled input, Enter applies via backend, note counts", async ({ page }) => {
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);
  await cell.dblclick();
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await expect(input).toHaveValue(before);
  await input.fill(`${before} EDITED`);
  await input.press("Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(cell).toContainText("EDITED");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);
});

test("Esc cancels with zero change and no notes", async ({ page }) => {
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  await cell.dblclick();
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill("SHOULD-NOT-STICK");
  await input.press("Escape");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(cell).toHaveText(before);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
});

test("orphan null cell is a no-op", async ({ page }) => {
  const orphanRow = page.locator('.tv-row[data-kind="leftOnly"]').first();
  await expect(orphanRow).toBeVisible();
  const nullCell = orphanRow.locator(".tv-cell-empty");
  await expect(nullCell).toHaveCount(1);
  await nullCell.dblclick();
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
});

test("reload clears applied mock edits", async ({ page }) => {
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  await cell.dblclick();
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill(`${before} TEMP`);
  await input.press("Enter");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(1);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
});
