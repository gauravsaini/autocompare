/**
 * Wave-198 staged cell edits — full parity proof. MOCK IPC ONLY
 * (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * and pins the staged model that `editable-197` switched to:
 *  - commits stage LOCALLY (`edit-note`, dirty dot, row kind
 *    untouched, no `applied-note`) — no backend IPC at commit time;
 *  - Save replays staged edits per side via `text_apply_edit`, then
 *    `text_save_side` (receipt, `applied-note`, staged cleared);
 *  - a backend-rejected commit keeps staged text + honest
 *    `edit-error`; a Save-flush replay failure pauses Save (no disk
 *    write, staged kept); a Save failure keeps staged;
 *  - the hunks slot is fixed-width (`min-w-[14ch]`) so selecting a
 *    hunk ("8 hunks" -> "hunk N of 8") never shifts the Save group.
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

async function stageLeftEdit(page: Page, suffix: string) {
  const { left } = await firstChangedCells(page);
  const before = (await left.textContent()) ?? "";
  await left.focus();
  await left.press("Enter");
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill(`${before} ${suffix}`);
  await input.press("Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(left).toContainText(suffix);
}

test("staged edits flush per side, Save receipt counts applied", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const { row, left, right } = await firstChangedCells(page);
  const rowId = await row.getAttribute("data-testid");
  const leftBefore = (await left.textContent()) ?? "";
  const rightBefore = (await right.textContent()) ?? "";

  await left.focus();
  await left.press("Enter");
  const leftInput = page.getByTestId("textview/cell-edit");
  await expect(leftInput).toBeVisible();
  await leftInput.fill(`${leftBefore} STAGE198-L`);
  await leftInput.press("Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);

  await right.focus();
  await right.press("Enter");
  const rightInput = page.getByTestId("textview/cell-edit");
  await expect(rightInput).toBeVisible();
  await rightInput.fill(`${rightBefore} STAGE198-R`);
  await rightInput.press("Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);

  // Both sides staged, nothing applied, row kind untouched.
  await expect(page.getByTestId("textview/edit-note")).toContainText("2 local edits");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await expect(page.getByTestId(rowId as string)).toHaveAttribute("data-kind", "changed");

  // Saving left flushes only the left edit.
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/save-error")).toHaveCount(0);

  // Saving right flushes the remainder.
  await page.getByTestId("textview/save-right").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved right to");
  await expect(page.getByTestId("textview/applied-note")).toContainText("2 edits applied");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("backend-rejected commit keeps staged text with honest edit-error", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_APPLY_EDIT_NEXT__ = {
      mode: "failure",
      error: "mock apply failed 198",
    };
  });
  await stageLeftEdit(page, "REJECT198");

  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/edit-error")).toContainText("mock apply failed 198");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);

  // Retry via Save: the replay uses the real mock backend (the forced
  // failure was consumed once at commit time), so it lands honestly.
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/edit-error")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("Save-flush replay failure pauses Save, staged kept", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await stageLeftEdit(page, "PAUSE198");

  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");

  // Plant the failure so the Save-time replay (not the commit) breaks.
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_APPLY_EDIT_NEXT__ = {
      mode: "failure",
      error: "replay break 198",
    };
  });
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-error")).toContainText("paused");
  await expect(page.getByTestId("textview/edit-error")).toContainText("replay break 198");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);

  // Retry with a healthy backend: the staged edit flushes honestly.
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("Save failure keeps staged edits with honest error", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await stageLeftEdit(page, "SAVEFAIL198");

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "failure",
      error: "disk full 198",
    };
  });
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-error")).toContainText("Save failed");
  await expect(page.getByTestId("textview/save-error")).toContainText("disk full 198");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("hunks slot is fixed-width, selection never shifts Save group", async ({ page }) => {
  const { pageErrors } = trackErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const hunks = page.getByTestId("textview/toolbar-hunks");
  await expect(hunks).toBeVisible();
  // Fixed-width parking slot (wave-198): reserves "hunk 100 of 100".
  await expect(hunks).toHaveClass(/min-w-\[14ch\]/);
  await expect(hunks).toHaveClass(/shrink-0/);

  const saveLeft = page.getByTestId("textview/save-left");
  await expect(saveLeft).toBeVisible();
  const before = await saveLeft.boundingBox();
  expect(before).not.toBeNull();

  // Select a changed row: the label switches to "hunk N of M".
  const row = page.locator('.tv-row[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  await row.click();
  await expect(hunks).toContainText(/hunk \d+ of \d+/);

  const after = await saveLeft.boundingBox();
  expect(after).not.toBeNull();
  // No horizontal shift of the Save group mid-gesture (1px tolerance).
  expect(Math.abs((after as { x: number }).x - (before as { x: number }).x)).toBeLessThanOrEqual(1);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});
