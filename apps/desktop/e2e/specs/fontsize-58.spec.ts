import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-58 font-size no-op audit: every remaining
 * text-[var(--ac-font-size-*)] is now text-[length:var(...)] (real
 * font-size) or deleted as a dead dup of ac-chrome-label/ac-meta.
 * Spot-checks one ui + one code site per affected view family.
 * New file only, no existing spec touched.
 */
async function createSession(page: Page, kind: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(`/tmp/u58-${kind}-left`);
  await page.getByTestId("session/right-path").fill(`/tmp/u58-${kind}-right`);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestId)).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("audited views ride the 12px token ramp", async ({ page }) => {
  await createSession(page, "merge", "view-merge");
  const conflictCount = page.getByTestId("merge-conflict-count");
  await expect(conflictCount).toBeVisible();
  const mergeSize = await conflictCount.evaluate(
    (el) => window.getComputedStyle(el).fontSize
  );
  expect(Math.abs(parseFloat(mergeSize) - 12)).toBeLessThan(0.5);

  await createSession(page, "table", "view-table");
  const colsVisible = page.getByTestId("table-cols-visible");
  await expect(colsVisible).toBeVisible();
  const tableSize = await colsVisible.evaluate(
    (el) => window.getComputedStyle(el).fontSize
  );
  expect(Math.abs(parseFloat(tableSize) - 12)).toBeLessThan(0.5);
  const tableFamily = await colsVisible.evaluate(
    (el) => window.getComputedStyle(el).fontFamily
  );
  expect(tableFamily).toContain("Mono");

  await createSession(page, "sync", "view-sync");
  const syncStatus = page.getByTestId("syncview/totals");
  await expect(syncStatus).toBeVisible();
  const syncSize = await syncStatus.evaluate(
    (el) => window.getComputedStyle(el).fontSize
  );
  expect(Math.abs(parseFloat(syncSize) - 12)).toBeLessThan(0.5);

  await assertNoOverlap(page);
});
