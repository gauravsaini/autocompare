/**
 * Wave-137 row Enter-to-edit (BC parity, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-boot demo text diff
 * (real engine rows) and proves the row flow only:
 *  - j/k or click selects a row, Enter opens that row's cell editor
 *    (left cell first when both non-null; orphan side skips to the
 *    text side);
 *  - typed text commits through the unchanged 66V backend path
 *    (Ctrl/Cmd+Enter + single-line Enter still commit);
 *  - Esc cancels with zero change and focus returns to the cell so
 *    j/k continues;
 *  - orphan null side never opens an editor (no-op).
 *
 * New file only, no existing spec touched. Skip policy matches the
 * other specs.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function bootText(page: Page): Promise<void> {
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
}

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  return { pageErrors, consoleErrors };
}

function expectZeroNoise(pageErrors: string[], consoleErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

async function rowIndex(row: Locator): Promise<number> {
  const testid = (await row.getAttribute("data-testid")) ?? "";
  return Number(testid.replace("textview/row-", ""));
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("row Enter opens left editor prefilled, Ctrl+Enter commits via backend", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const row = page.locator('.tv-row[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  const leftCell = row.locator(".tv-cell").first();
  const before = (await leftCell.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);

  // Keyboard select (j/k flow owns the row), then Enter opens the
  // LEFT cell editor prefilled — both sides non-null here.
  await row.click();
  await expect(row).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await row.press("Enter");
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await expect(editor).toHaveValue(before);
  // The editor lives in the left cell, not the right one.
  await expect(row.locator(".tv-cell").first().getByTestId("textview/cell-edit")).toHaveCount(1);
  await expect(row.locator(".tv-cell").nth(1).getByTestId("textview/cell-edit")).toHaveCount(0);

  // Unchanged 66V commit path: Ctrl+Enter applies through the backend.
  await editor.fill(`${before} ROWEDIT-137`);
  await editor.press("Control+Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(leftCell).toContainText("ROWEDIT-137");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);

  expectZeroNoise(pageErrors, consoleErrors);
});

test("row Enter on orphan opens text side, Esc cancels + j/k continues", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // leftOnly = right cell is the orphan null: Enter must skip it and
  // open the left (text) side.
  const row = page.locator('.tv-row[data-kind="leftOnly"]').first();
  await expect(row).toBeVisible();
  await expect(row.locator(".tv-cell-empty")).toHaveCount(1);
  const textCell = row.locator(".tv-cell").first();
  const before = (await textCell.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);

  await row.click();
  await expect(row).toHaveAttribute("data-selected", "true");
  await row.press("Enter");
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await expect(editor).toHaveValue(before);

  // Esc cancels with zero change, focus returns to the cell, and j/k
  // continues from there (bubbles to the row handler).
  await editor.fill("SHOULD-NOT-STICK-137");
  await editor.press("Escape");
  await expect(editor).toHaveCount(0);
  await expect(textCell).toHaveText(before);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await expect(textCell).toBeFocused();

  const idx = await rowIndex(row);
  await page.keyboard.press("ArrowDown");
  const downIdx = idx + 1;
  await expect(page.getByTestId(`textview/row-${downIdx}`)).toHaveAttribute("data-selected", "true");
  await expect(page.locator('[data-testid^="textview/row-"][data-selected="true"]')).toHaveCount(1);

  expectZeroNoise(pageErrors, consoleErrors);
});

test("row Enter on rightOnly orphan opens right side, single-line Enter commits", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // rightOnly = left cell is the orphan null: Enter must skip it and
  // open the right (text) side. Proves the unchanged single-line
  // Enter commit path through the row flow.
  const row = page.locator('.tv-row[data-kind="rightOnly"]').first();
  await expect(row).toBeVisible();
  await expect(row.locator(".tv-cell-empty")).toHaveCount(1);
  const textCell = row.locator(".tv-cell").nth(1);
  const before = (await textCell.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);

  await row.click();
  await expect(row).toHaveAttribute("data-selected", "true");
  await row.press("Enter");
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await expect(editor).toHaveValue(before);
  // The orphan left cell holds no editor.
  await expect(row.locator(".tv-cell").first().getByTestId("textview/cell-edit")).toHaveCount(0);

  await editor.fill(`${before} R137`);
  await editor.press("Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(textCell).toContainText("R137");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");

  expectZeroNoise(pageErrors, consoleErrors);
});
