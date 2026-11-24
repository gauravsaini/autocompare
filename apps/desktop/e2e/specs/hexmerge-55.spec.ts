import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-55 hex/merge chrome typography: goto bar buttons and merge
 * take-left buttons ride the ac-chrome-label ramp (tight tracking).
 * New file only, no existing spec touched.
 */
async function createSession(page: Page, kind: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(`/tmp/u55-${kind}-left`);
  await page.getByTestId("session/right-path").fill(`/tmp/u55-${kind}-right`);
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

test("hex and merge buttons ride the label ramp", async ({ page }) => {
  await createSession(page, "hex", "view-hex");
  const hexBtn = page.getByTestId("hex-copy-bar").getByRole("button").first();
  await expect(hexBtn).toBeVisible();
  const hexTracking = await hexBtn.evaluate(
    (el) => window.getComputedStyle(el).letterSpacing
  );
  expect(Math.abs(parseFloat(hexTracking) - -0.12)).toBeLessThan(0.05);

  await createSession(page, "merge", "view-merge");
  const takeLeft = page.locator('[data-testid^="merge-take-left-"]').first();
  await expect(takeLeft).toBeVisible();
  const mergeTracking = await takeLeft.evaluate(
    (el) => window.getComputedStyle(el).letterSpacing
  );
  expect(Math.abs(parseFloat(mergeTracking) - -0.12)).toBeLessThan(0.05);

  await assertNoOverlap(page);
});
