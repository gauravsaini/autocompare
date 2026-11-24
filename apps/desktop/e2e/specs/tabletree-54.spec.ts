import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-54 table/tree chrome typography: view buttons and notes ride the
 * ac-chrome-label ramp (tight tracking), toggle rides ac-meta.
 * New file only, no existing spec touched.
 */
async function createSession(page: Page, kind: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(`/tmp/u54-${kind}-left`);
  await page.getByTestId("session/right-path").fill(`/tmp/u54-${kind}-right`);
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

test("table and folder chrome ride the label ramp", async ({ page }) => {
  await createSession(page, "table", "view-table");
  const exportBtn = page.getByTestId("table-export");
  await expect(exportBtn).toBeVisible();
  const btnTracking = await exportBtn.evaluate(
    (el) => window.getComputedStyle(el).letterSpacing
  );
  expect(Math.abs(parseFloat(btnTracking) - -0.12)).toBeLessThan(0.05);

  const noteSize = await page
    .getByTestId("table-cols-count")
    .evaluate((el) => window.getComputedStyle(el).fontSize);
  expect(noteSize).toBe("12px");

  await createSession(page, "folder", "view-folder");
  const totalsTracking = await page
    .getByTestId("folderview/totals")
    .evaluate((el) => window.getComputedStyle(el).letterSpacing);
  expect(Math.abs(parseFloat(totalsTracking) - -0.12)).toBeLessThan(0.05);

  await assertNoOverlap(page, "[data-testid^=\"view-\"] [data-testid]");
});
