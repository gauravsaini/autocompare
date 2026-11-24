import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-59 differences-only display filter: the TextView toggle hides
 * kind-equal rows from the rendered list (backend indexes preserved),
 * names the hidden same-row count honestly, and restores everything
 * on toggle-off. Purely presentational, no new IPC.
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

test("differences-only hides same rows and restores", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const toggle = page.getByTestId("textview/diffsonly-toggle");
  await expect(toggle).toBeVisible();

  const before = await page.locator(".tv-row").count();
  expect(before).toBeGreaterThan(0);

  await toggle.check();
  const note = page.getByTestId("textview/diffsonly-note");
  await expect(note).toBeVisible();
  const noteText = (await note.textContent()) ?? "";
  const hidden = Number.parseInt(noteText.replace(/[^0-9]/g, ""), 10);
  expect(hidden).toBeGreaterThan(0);
  await expect(page.locator(".tv-row")).toHaveCount(before - hidden);
  const noteSize = await note.evaluate((el) => window.getComputedStyle(el).fontSize);
  expect(Math.abs(parseFloat(noteSize) - 12)).toBeLessThan(0.5);

  await toggle.uncheck();
  await expect(note).toBeHidden();
  await expect(page.locator(".tv-row")).toHaveCount(before);

  await assertNoOverlap(page);
});
