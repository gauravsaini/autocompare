/**
 * Wave-172b text merge BC parity: gutter/menu/shortcut copy stages in
 * memory, disk write only on explicit Save. MOCK IPC ONLY
 * (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, hunk starts at
 * rows 6, 20, 51, 98, ...) and proves:
 *  - stage-no-save: gutter `hunk-copy-left-2` (row 51) merges left to
 *    right in memory with NO disk write (planted
 *    `__TEXT_SAVE_SIDE_NEXT__` failure sentinel stays unconsumed),
 *    `textview/copy-note` says unsaved + names Save right, and
 *    nothing lands in localStorage (`ac:textcopy125:` stays absent);
 *  - Save persists: user Save right afterwards writes (forced success
 *    receipt) and clears the staged copy-note;
 *  - reload drops staged: after a gutter copy, `page.reload()` shows
 *    row 51 changed again with no copy-note (BC wins over wave-125
 *    reload persistence);
 *  - menu Take same path: selecting row 98 + `menu-event`
 *    `session.take-left` produces the same gutter receipt + status
 *    message, `session.take-right` mirrors;
 *  - shortcuts: `l`/`r` copy take-left/right, `n`/`p` jump hunks,
 *    Ctrl+F / Ctrl+G open find/goto, Esc closes them.
 *
 * PORT-FREE: runs on a fresh port (never 1420,
 * `reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate. Skip policy matches the
 * other specs (infra failures skip).
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

async function bootText(page: Page): Promise<void> {
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
}

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(String(msg.text()));
  });
  return { pageErrors, consoleErrors };
}

/** Plant a save override that screams if the copy path ever saves. */
async function plantSaveSentinel(page: Page): Promise<void> {
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "failure",
      error: "AUTOSAVE-PROBE-172B",
    };
  });
}

/** The copy path must never consume the planted save override. */
async function expectSaveUntouched(page: Page): Promise<void> {
  const leftover = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__,
  );
  expect((leftover as { error?: unknown } | null)?.error).toBe("AUTOSAVE-PROBE-172B");
}

async function storedCopyKeys(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const keys: string[] = [];
    for (let i = 0; i < window.localStorage.length; i += 1) {
      const k = window.localStorage.key(i);
      if (k !== null && k.startsWith("ac:textcopy125:")) keys.push(k);
    }
    return keys;
  });
}

test("gutter copy stages in memory with no disk write and no localStorage", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const rightOf51 = page.getByTestId("textview/row-51").locator('div[title="right"]');
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await expect(page.getByTestId("hunk-copy-left-2")).toBeVisible();

  await plantSaveSentinel(page);
  await page.getByTestId("hunk-copy-left-2").click();

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("1 of 1 row(s) merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-172B");
  await expect(note).not.toContainText("saved right to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page);

  // Hunk resolves in memory: row 51 equal, right carries the left text.
  const leftOf51 = page.getByTestId("textview/row-51").locator('div[title="left"]');
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(rightOf51).toHaveText((await leftOf51.textContent()) ?? "");

  // BC stage-only: nothing is journaled for reload replay.
  expect(await storedCopyKeys(page)).toEqual([]);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("explicit Save persists the staged copy and clears staged state", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  await plantSaveSentinel(page);
  await page.getByTestId("hunk-copy-left-2").click();
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expectSaveUntouched(page);

  await page.evaluate((p) => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "success",
      path: p,
      lines: 644,
      bytes: 4242,
    };
  }, "/tmp/mock-172b-right.txt");
  await page.getByTestId("textview/save-right").click();

  const receipt = page.getByTestId("textview/save-note");
  await expect(receipt).toContainText("Saved right to /tmp/mock-172b-right.txt");
  await expect(receipt).toContainText("4242 bytes");
  // Staged set for the saved side is flushed: the staged note is gone.
  await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("reload drops the staged copy (BC wins over wave-125 persistence)", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("hunk-copy-left-2").click();
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // Staged merge did NOT survive: hunk is a difference again, no note.
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed", { timeout: 15000 });
  await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menu Take left/right ride the same gutter path (validated)", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // No hunk selected yet: the menu path parks honestly (bridge Meta+D).
  await page.keyboard.press("Meta+d");
  await expect(page.getByTestId("statusbar/message")).toContainText("needs a hunk row selected");

  // Take-left on the row-98 changed hunk via the native-menu bridge.
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-98").click();
  await plantSaveSentinel(page);
  await page.keyboard.press("Meta+d");

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 4 take-left");
  await expect(note).toContainText("1 of 1 row(s) merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-172B");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");
  await expectSaveUntouched(page);

  // Take-right (bridge Meta+E) mirrors on the row-51 hunk.
  const leftOf51 = page.getByTestId("textview/row-51").locator('div[title="left"]');
  const rightOf51 = page.getByTestId("textview/row-51").locator('div[title="right"]');
  const rightText = (await rightOf51.textContent()) ?? "";
  expect(rightText.length).toBeGreaterThan(0);
  await page.getByTestId("textview/row-51").click();
  await page.keyboard.press("Meta+e");
  await expect(note).toContainText("take-right");
  await expect(note).toContainText("press Save left");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-172B");
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(leftOf51).toHaveText(rightText);
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-right requested for row 51");
  await expectSaveUntouched(page);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("shortcuts: l/r copy, n/p jump, Ctrl+F/G open, Esc closes", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // n/p jump between hunks from a selected row.
  await page.getByTestId("textview/row-51").click();
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-selected", "true");
  await page.getByTestId("textview/row-51").focus();
  await page.keyboard.press("n");
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-selected", "true");
  await page.getByTestId("textview/row-98").focus();
  await page.keyboard.press("p");
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-selected", "true");

  // l copies take-left on the selected hunk.
  await plantSaveSentinel(page);
  await page.getByTestId("textview/row-51").focus();
  await page.keyboard.press("l");
  await expect(page.getByTestId("textview/copy-note")).toContainText("Copied hunk 3 take-left");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page);
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");

  // r copies take-right on another hunk.
  await page.getByTestId("textview/row-98").click();
  await page.getByTestId("textview/row-98").focus();
  await page.keyboard.press("r");
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
  await expect(page.getByTestId("textview/copy-note")).toContainText("press Save left");
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");

  // Ctrl+F opens find, Esc closes it.
  await page.keyboard.press("Control+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);

  // Ctrl+G opens goto, Esc closes it.
  await page.keyboard.press("Control+g");
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
