/**
 * Wave-67A text save-to-disk wiring proof. MOCK IPC ONLY
 * (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`) and proves the per-side
 * Save affordance in `TextView`:
 *  - success: a dirty staged edit (forced `text_apply_edit` failure)
 *    plus `__TEXT_SAVE_SIDE_NEXT__ = success` makes Save clear the
 *    dirty edit for that side and show the receipt
 *    (`textview/save-note` names path + bytes);
 *  - failure: `__TEXT_SAVE_SIDE_NEXT__ = failure` keeps the staged
 *    edit with an honest `textview/save-error`;
 *  - orphan: `__TEXT_SAVE_ORPHAN_SIDE__` makes Save a no-op with an
 *    honest note, edits untouched, no IPC.
 *
 * The mock adapter has no `text_save_side` engine (unknown commands
 * resolve `[]`), so the view exposes test-only window overrides
 * consumed once per save; production never sets them and always takes
 * the real `textSaveSide` IPC path (receipt shape validated, invalid
 * shapes surface as honest errors). New file only, no existing spec
 * touched. Skip policy matches the other specs.
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

async function firstChangedLeftCell(page: Page) {
  const row = page.locator('[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  return { row, cell: row.locator(".tv-cell").first() };
}

async function makeDirtyLeftEdit(page: Page, text: string) {
  await page.evaluate((msg) => {
    (window as unknown as Record<string, unknown>).__TEXT_APPLY_EDIT_NEXT__ = {
      mode: "failure",
      error: msg,
    };
  }, "mock apply failed 67a");
  const { cell } = await firstChangedLeftCell(page);
  await cell.dblclick();
  const input = page.getByTestId("textview/cell-edit");
  await expect(input).toBeVisible();
  await input.fill(text);
  await input.press("Enter");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
}

test("save success clears dirty and shows receipt (mock IPC)", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/save-left")).toBeVisible();
  await expect(page.getByTestId("textview/save-right")).toBeVisible();

  await makeDirtyLeftEdit(page, "DIRTY-67A-SUCCESS");

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "success",
      path: "/tmp/mock-left-67a.txt",
      lines: 10,
      bytes: 123,
    };
  });
  await page.getByTestId("textview/save-left").click();
  const receipt = page.getByTestId("textview/save-note");
  await expect(receipt).toContainText("Saved left to /tmp/mock-left-67a.txt");
  await expect(receipt).toContainText("123 bytes");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/save-error")).toHaveCount(0);

  // Dark scheme: receipt + affordances hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(200);
  await expect(receipt).toBeVisible();
  await expect(page.getByTestId("textview/save-left")).toBeVisible();
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("save failure keeps staged edits with honest error", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await makeDirtyLeftEdit(page, "DIRTY-67A-FAIL");

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "failure",
      error: "disk full 67a",
    };
  });
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-error")).toContainText("Save failed");
  await expect(page.getByTestId("textview/save-error")).toContainText("disk full 67a");
  // Staged edit survives the failed save.
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("orphan-side save is a no-op with honest note", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await makeDirtyLeftEdit(page, "DIRTY-67A-ORPHAN");

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_ORPHAN_SIDE__ = "left";
  });
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-error")).toContainText("nothing to save");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  // Orphan never clears staged edits and never crashes.
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});
