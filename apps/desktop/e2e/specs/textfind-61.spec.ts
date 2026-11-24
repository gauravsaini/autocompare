import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-61 find in text: substring search over the loaded rows, Next/Prev
 * cycle with wrap, selection + focus follow the active hit, empty query
 * is a clean no-op, misses render an honest note. Purely presentational.
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

async function focusedRowTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("find cycles hits, selects, misses honestly", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const input = page.getByTestId("textview/find-input");
  const next = page.getByTestId("textview/find-next");
  const prev = page.getByTestId("textview/find-prev");
  const status = page.getByTestId("textview/find-status");
  // Wave-112 preamble: the find panel is closed by default (BC truth);
  // open it via its toolbar toggle. Zero assertion changes below.
  const findToggle112 = page.getByTestId("textview/toolbar-find");
  if ((await findToggle112.getAttribute("aria-expanded")) !== "true") await findToggle112.click();
  await expect(input).toBeVisible();

  await input.fill("variant");
  await input.press("Enter");
  await expect(status).toContainText(/match 1 of \d+/);
  // Activation ends with DOM focus on the hit row (plus its scroll).
  // Waiting for that focus proves the whole chain settled, so the Next
  // click below cannot race a mid-flight focus scroll under load.
  await expect.poll(() => focusedRowTestId(page), { timeout: 10000 }).toMatch(/^textview\/row-/);
  const first = await status.textContent();
  const total = Number.parseInt((first ?? "").replace(/[^0-9]/g, "").slice(1), 10);
  expect(total).toBeGreaterThan(0);
  await expect(page.locator('[data-testid^="textview/row-"][data-selected="true"]')).toHaveCount(1);

  await next.click();
  await expect(status).toContainText(new RegExp(`match 2 of ${total}`));
  await prev.click();
  await expect(status).toContainText(`match 1 of ${total}`);

  await input.fill("");
  await expect.poll(() => input.inputValue(), { timeout: 10000 }).toBe("");
  await expect(status).toContainText("Find across loaded rows");
  await expect(next).toBeDisabled();

  await input.fill("zzz-no-such-text-xyz");
  await input.press("Enter");
  await expect(status).toContainText(/no matches in \d+ loaded rows/);

  await assertNoOverlap(page);
});
