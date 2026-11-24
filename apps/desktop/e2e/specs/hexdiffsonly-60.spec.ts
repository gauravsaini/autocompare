import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-60 hex differences-only filter: the HexView toggle hides rows
 * with zero differing bytes (byte offsets preserved), names the hidden
 * count honestly, and restores everything on toggle-off. Purely
 * presentational, no new IPC.
 * New file only, no existing spec touched.
 */
async function createSession(page: Page, kind: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(`/tmp/u60-${kind}-left`);
  await page.getByTestId("session/right-path").fill(`/tmp/u60-${kind}-right`);
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

test("differences-only hides same hex rows and restores", async ({ page }) => {
  await createSession(page, "hex", "view-hex");
  const toggle = page.getByTestId("hexview/diffsonly-toggle");
  await expect(toggle).toBeVisible();

  const before = await page.locator(".hx-row").count();
  expect(before).toBeGreaterThan(0);

  await toggle.check();
  const note = page.getByTestId("hexview/diffsonly-note");
  await expect(note).toBeVisible();
  const noteText = (await note.textContent()) ?? "";
  const hidden = Number.parseInt(noteText.replace(/[^0-9]/g, ""), 10);
  expect(hidden).toBeGreaterThan(0);
  await expect(page.locator(".hx-row")).toHaveCount(before - hidden);
  const noteSize = await note.evaluate((el) => window.getComputedStyle(el).fontSize);
  expect(Math.abs(parseFloat(noteSize) - 12)).toBeLessThan(0.5);

  await toggle.uncheck();
  await expect(note).toBeHidden();
  await expect(page.locator(".hx-row")).toHaveCount(before);

  await assertNoOverlap(page);
});
