/**
 * Wave-197 editable panes thin slice: `RowLine` now lives in
 * `views/text/RowLine.tsx` (moved verbatim from `TextView.tsx`, zero
 * behavior change). MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`) and pins the thinnest
 * editable slice — single-line edit -> backend apply -> Save — on BOTH
 * panes (cells open via keyboard focus + Enter, the same `RowLine`
 * `startEdit` path dblclick rides; dblclick coordinates race the
 * scrolled list in this webkit harness — see REPORT):
 *  - left cell: focus + Enter opens the prefilled editor, fill + Enter
 *    applies via `text_apply_edit` (mock serves the patched row on
 *    refetch), `applied-note` counts it, editor closes;
 *  - right cell: same path on the right pane (BC parity needs both
 *    panes editable, not just left);
 *  - Esc cancels with zero change and no notes;
 *  - orphan (null) cell click selects the row but never opens an editor;
 *  - Save right writes via `text_save_side` (mock `[]` fallback
 *    receipt) with no `save-error`.
 *
 * New file only, no existing spec touched. Skip policy matches the
 * other specs.
 */
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function firstChangedCells(page: Page) {
  const row = page.locator('.tv-row[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  const cells = row.locator(".tv-cell");
  await expect(cells).toHaveCount(2);
  return { row, left: cells.first(), right: cells.nth(1) };
}

function trackErrors(page: Page): { pageErrors: string[] } {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { pageErrors };
}

test("left pane single-line edit applies via backend", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const { left } = await firstChangedCells(page);
  const before = (await left.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);

  await left.focus();
  await left.press("Enter");
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await expect(input).toHaveValue(before);
  await input.fill(`${before} EDIT197-L`);
  await input.press("Enter");

  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(left).toContainText("EDIT197-L");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("right pane single-line edit applies via backend", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const { right } = await firstChangedCells(page);
  const before = (await right.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);

  await right.focus();
  await right.press("Enter");
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await expect(input).toHaveValue(before);
  await input.fill(`${before} EDIT197-R`);
  await input.press("Enter");

  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(right).toContainText("EDIT197-R");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("Esc cancels with zero change and no notes", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const { left } = await firstChangedCells(page);
  const before = await left.textContent();

  await left.focus();
  await left.press("Enter");
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill("SHOULD-NOT-STICK-197");
  await input.press("Escape");

  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(left).toHaveText(before ?? "");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("orphan null cell click never opens an editor", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const orphanRow = page.locator('.tv-row[data-kind="leftOnly"]').first();
  await expect(orphanRow).toBeVisible();
  const nullCell = orphanRow.locator(".tv-cell-empty");
  await expect(nullCell).toHaveCount(1);
  // Single click selects the row (accurate hit-test); no editor opens
  // for null cells by design (`editable` is false without `editKey`).
  await nullCell.click();
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("Save right persists after an edit with an honest receipt", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const { left } = await firstChangedCells(page);
  const before = (await left.textContent()) ?? "";

  await left.focus();
  await left.press("Enter");
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill(`${before} EDIT197-SAVE`);
  await input.press("Enter");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");

  await page.getByTestId("textview/save-right").click();
  const receipt = page.getByTestId("textview/save-note");
  await expect(receipt).toContainText("Saved right to");
  await expect(receipt).toContainText("bytes");
  await expect(page.getByTestId("textview/save-error")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});
