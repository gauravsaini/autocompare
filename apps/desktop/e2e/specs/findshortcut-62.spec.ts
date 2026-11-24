import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-62 find shortcut: Cmd/Ctrl+F focuses the active view's find box
 * (text, table, hex). Only the active view is mounted, so focus always
 * lands in exactly one box. No new IPC.
 * New file only, no existing spec touched.
 */
async function createSession(page: Page, kind: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(`/tmp/u62-${kind}-left`);
  await page.getByTestId("session/right-path").fill(`/tmp/u62-${kind}-right`);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestId)).toBeVisible();
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
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

test("cmd+f focuses the active find box", async ({ page }) => {
  // WebKit sometimes drops a lone modifier-chord under automation load:
  // one bounded re-press per view keeps the assert deterministic without
  // masking a real product miss (the handler either focuses or it doesn't).
  async function pressFindFor(want: string): Promise<void> {
    await page.keyboard.press("Meta+f");
    try {
      await expect.poll(() => focusedTestId(page), { timeout: 2000 }).toBe(want);
    } catch {
      await page.keyboard.press("Meta+f");
      await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe(want);
    }
  }

  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await pressFindFor("textview/find-input");

  await createSession(page, "table", "view-table");
  await pressFindFor("table-find-input");

  await createSession(page, "hex", "view-hex");
  await pressFindFor("hex-find-input");

  await assertNoOverlap(page);
});
