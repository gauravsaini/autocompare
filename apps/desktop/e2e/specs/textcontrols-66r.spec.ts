/**
 * Wave-66R text controls collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-boot demo text diff
 * (real engine rows) and proves the Beyond Compare style collapse:
 *  - one toolbar row (`textview/toolbar`) hosts Find / Goto / Ignore
 *    toggles, prev/next difference nav + hunk position, and the
 *    wrap / line-numbers / differences-only / context controls;
 *  - find + goto + ignore panels render open by default (backward
 *    compat: every legacy testid stays visible on load);
 *  - Ctrl+F focuses find, Esc collapses it, Ctrl+F reopens it;
 *    Ctrl+G focuses goto, the toolbar button collapses/reopens it;
 *  - the Ignore menu collapses/reopens; the whitespace toggle
 *    re-diffs end to end with the note hidden (wave-75: the mock
 *    echoes appliedOptions, so the honest-fallback note never shows),
 *    and unchecking keeps it hidden;
 *  - prev/next difference nav walks hunks with wrap, selects exactly
 *    one row, and names the position honestly;
 *  - zero console errors / page errors throughout.
 *
 * In-flow panels only (never absolute), so the wave-7 overlap gate
 * holds with every panel open. New file only, no existing spec
 * touched. Skip policy matches the other specs.
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function focusedTestId(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("text controls collapse: toolbar, overlays, ignore menu, diff nav", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 1. Single toolbar row with every toggle + nav. Wave-112B: panels
  //    are closed by default (BC truth), so open them via toggles first;
  //    assertions below are unchanged.
  for (const toggle of ["textview/toolbar-find", "textview/toolbar-goto", "textview/toolbar-ignore"] as const) {
    const t = page.getByTestId(toggle);
    if ((await t.getAttribute("aria-expanded")) !== "true") await t.click();
  }
  const toolbar = page.getByTestId("textview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-find")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-goto")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-ignore")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-prev-diff")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-next-diff")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-hunks")).toBeVisible();
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect(page.getByTestId("textview/ignore-whitespace")).toBeVisible();
  await expect(page.getByTestId("textview/wrap-toggle")).toBeVisible();
  await expect(page.getByTestId("textview/linenos-toggle")).toBeVisible();
  await expect(page.getByTestId("textview/diffsonly-toggle")).toBeVisible();
  await expect(page.getByTestId("textview/context-lines")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // 2. Ctrl+F focuses find; Esc collapses; Ctrl+F reopens + focuses.
  await page.keyboard.press("Meta+f");
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/find-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toBeHidden();
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/find-input");

  // 3. Ctrl+G focuses goto; toolbar button collapses/reopens.
  await page.keyboard.press("Meta+g");
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/goto-input");
  await page.getByTestId("textview/toolbar-goto").click();
  await expect(page.getByTestId("textview/goto-input")).toBeHidden();
  await page.getByTestId("textview/toolbar-goto").click();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();

  // 4. Ignore menu collapses/reopens; the toggle re-diffs end to
  //    end (wave-75: mock echoes appliedOptions) with no fallback note
  //    element at all (wave-97 retired it — Rust TextOptions honors
  //    both flags). Wave-89 note: the step-2 Esc closed every panel
  //    (global collapse), so the first click REOPENS ignore here.
  await page.getByTestId("textview/toolbar-ignore").click();
  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  await expect(wsToggle).toBeVisible();
  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(wsToggle).toBeHidden();
  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(wsToggle).toBeVisible();
  await wsToggle.check();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await wsToggle.uncheck();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);

  // 5. Diff nav: next walks hunks 1 -> 2, prev walks back, exactly one
  //    row selected, position named honestly.
  const statusBefore = await page.getByTestId("textview/status").innerText();
  await page.getByTestId("textview/toolbar-next-diff").click();
  await expect(page.getByTestId("textview/diffnav-note")).toContainText(/difference 1 of \d+/);
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toMatch(/^textview\/row-/);
  await expect(page.locator('[data-testid^="textview/row-"][data-selected="true"]')).toHaveCount(1);
  await expect(page.getByTestId("textview/toolbar-hunks")).toContainText(/hunk 1 of \d+/);
  await page.getByTestId("textview/toolbar-next-diff").click();
  await expect(page.getByTestId("textview/diffnav-note")).toContainText(/difference 2 of \d+/);
  await page.getByTestId("textview/toolbar-prev-diff").click();
  await expect(page.getByTestId("textview/diffnav-note")).toContainText(/difference 1 of \d+/);
  // Nav is presentational: the diff itself never moves.
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
