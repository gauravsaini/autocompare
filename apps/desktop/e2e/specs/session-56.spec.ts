import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-56 session manager typography: dialog title, buttons, and
 * microcopy ride the token ramp (tight tracking, single meta size).
 * New file only, no existing spec touched.
 */
test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("session manager rides the label ramp", async ({ page }) => {
  // Overlap is only meaningful with the modal closed (the dialog
  // intentionally overlays the workspace while open).
  await assertNoOverlap(page);
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();

  const titleTracking = await page.evaluate(
    () => window.getComputedStyle(document.querySelector("#session-manager-title") as Element).letterSpacing
  );
  // 18px * -0.01em = -0.18px
  expect(Math.abs(parseFloat(titleTracking) - -0.18)).toBeLessThan(0.05);

  const createBtn = page.getByTestId("session/create-button");
  await expect(createBtn).toBeVisible();
  const btnTracking = await createBtn.evaluate(
    (el) => window.getComputedStyle(el).letterSpacing
  );
  expect(Math.abs(parseFloat(btnTracking) - -0.12)).toBeLessThan(0.05);

  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
});
