/**
 * Wave-66V backend apply-edit + re-compare proof. MOCK IPC ONLY
 * (`VITE_MOCK_IPC=1`): the mock `text_apply_edit` patches the served
 * rows and recomputes the edited row kind, so the UI re-fetch proves
 * the wiring; engine regroup semantics are proven by the Rust unit
 * tests (`apply_edit_recompares_edited_row_to_equal`).
 *
 *  - editing a changed row's left cell to match its right text turns
 *    the row `equal` after refetch (kind attribute flips, highlights
 *    clear, applied-note counts);
 *  - editing to still-different text keeps the row `changed` with the
 *    new text visible.
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

async function firstChangedRow(page: import("@playwright/test").Page) {
  const row = page.locator('[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  const leftCell = row.locator(".tv-cell").first();
  const cells = row.locator(".tv-cell");
  const rightCell = cells.nth(1);
  return { row, leftCell, rightCell };
}

test("edit left to match right turns the row equal", async ({ page }) => {
  const { row, leftCell, rightCell } = await firstChangedRow(page);
  const rowId = await row.getAttribute("data-testid");
  expect(rowId).toMatch(/textview\/row-\d+/);
  const rightText = ((await rightCell.textContent()) ?? "").trim();
  expect(rightText.length).toBeGreaterThan(0);
  await leftCell.dblclick();
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill(rightText);
  await input.press("Enter");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  // Refetch regroups: the edited row itself flips to equal and its
  // inline highlights clear.
  const edited = page.getByTestId(rowId as string);
  await expect(edited).toHaveAttribute("data-kind", "equal");
  await expect(edited.locator('[data-testid="inline/hl"]')).toHaveCount(0);
});

test("edit to still-different text keeps the row changed", async ({ page }) => {
  const { leftCell } = await firstChangedRow(page);
  await leftCell.dblclick();
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill("STILL-DIFFERENT-66V");
  await input.press("Enter");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.locator('[data-kind="changed"]', { hasText: "STILL-DIFFERENT-66V" })).toHaveCount(1);
});
