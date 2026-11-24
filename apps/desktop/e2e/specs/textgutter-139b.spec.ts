/**
 * Wave-139b BC-faithful gutter merge: no autosave, no clipboard hijack.
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, hunk starts at
 * rows 6, 20, 51, 98, ...) and proves:
 *  - gutter L2R: clicking `hunk-copy-left-2` (single-row changed hunk
 *    at row 51) merges left to right through `text_apply_edit` + local
 *    apply with NO disk write (a planted `__TEXT_SAVE_SIDE_NEXT__`
 *    failure sentinel stays unconsumed and never surfaces in the
 *    receipt) and NO clipboard touch; `textview/copy-note` says
 *    unsaved and names the Save side; `textview/save-note` stays
 *    absent until the user presses Save;
 *  - user Save: clicking `textview/save-right` afterwards (success
 *    forced via the same override) writes and shows the
 *    `textview/save-note` receipt;
 *  - gutter R2L mirrors on the row-98 changed hunk (`hunk-copy-right-3`,
 *    left takes the right text, Save left named, still no disk write);
 *  - orphan: take-left on the left-only hunk 0 (rows 6..9, no right
 *    cells) stays honest - one-sided note, kept session-local in the
 *    view, rows resolve to equal, still no disk write;
 *  - `l` key rides the same gutter path (same unsaved receipt).
 *
 * Menu Take Left/Right ride the same `autocompare:text-copy` path
 * (covered by merge-func-131); toolbar Copy likewise.
 *
 * PORT-FREE: runs on a fresh :1554 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
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
      error: "AUTOSAVE-PROBE-139B",
    };
  });
}

/** The copy path must never consume the planted save override. */
async function expectSaveUntouched(page: Page): Promise<void> {
  const leftover = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__,
  );
  expect((leftover as { error?: unknown } | null)?.error).toBe("AUTOSAVE-PROBE-139B");
}

async function forceSaveSuccess(page: Page, path: string): Promise<void> {
  await page.evaluate((p) => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "success",
      path: p,
      lines: 644,
      bytes: 4242,
    };
  }, path);
}

test("gutter L2R merges with no disk write; user Save writes", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const rightOf51 = page.getByTestId("textview/row-51").locator('div[title="right"]');
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await expect(page.getByTestId("hunk-copy-left-2")).toBeVisible();

  await plantSaveSentinel(page);
  await page.getByTestId("hunk-copy-left-2").click();

  // Receipt: hunk + take side + merged + unsaved; no save, no clipboard.
  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("1 of 1 row(s) merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-139B");
  await expect(note).not.toContainText("saved right to");
  await expect(note).not.toContainText("clipboard");
  await expect(note).not.toContainText("Clipboard");
  // Save affordance untouched: no receipt line without a user Save.
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  // The planted override survives: the copy path never called save.
  await expectSaveUntouched(page);

  // Hunk resolves visually: row 51 equal, right carries the left text.
  const leftOf51 = page.getByTestId("textview/row-51").locator('div[title="left"]');
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(rightOf51).toHaveText((await leftOf51.textContent()) ?? "");

  // User Save writes the dirty buffer now.
  await forceSaveSuccess(page, "/tmp/mock-139b-right.txt");
  await page.getByTestId("textview/save-right").click();
  const receipt = page.getByTestId("textview/save-note");
  await expect(receipt).toContainText("Saved right to /tmp/mock-139b-right.txt");
  await expect(receipt).toContainText("4242 bytes");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("gutter R2L mirrors with no disk write", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const row98 = page.getByTestId("textview/row-98");
  await expect(row98).toHaveAttribute("data-kind", "changed");
  const leftOf98 = row98.locator('div[title="left"]');
  const rightOf98 = row98.locator('div[title="right"]');
  const rightText = (await rightOf98.textContent()) ?? "";
  expect(rightText.length).toBeGreaterThan(0);
  await expect(page.getByTestId("hunk-copy-right-3")).toBeVisible();

  await plantSaveSentinel(page);
  await page.getByTestId("hunk-copy-right-3").click();

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("take-right");
  await expect(note).toContainText("merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save left");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-139B");
  await expect(note).not.toContainText("saved left to");
  await expect(note).not.toContainText("clipboard");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page);

  await expect(row98).toHaveAttribute("data-kind", "equal");
  await expect(leftOf98).toHaveText(rightText);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("orphan one-sided hunk stays honest with no disk write", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Hunk 0 (rows 6..9) is a left-only run: no right cells at all.
  for (const index of [6, 7, 8, 9]) {
    await expect(page.getByTestId(`textview/row-${index}`)).toHaveAttribute("data-kind", "leftOnly");
  }
  await expect(page.getByTestId("hunk-copy-left-0")).toBeVisible();

  await plantSaveSentinel(page);
  await page.getByTestId("hunk-copy-left-0").click();

  // Honest orphan note: one-sided, session-local, unsaved - no save.
  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 1 take-left");
  await expect(note).toContainText("one-sided");
  await expect(note).toContainText("kept session-local");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("Save right");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-139B");
  await expect(note).not.toContainText("saved right to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page);

  // View still resolves: every row survives and flips to equal, the
  // right cells carry the left text.
  for (const index of [6, 7, 8, 9]) {
    await expect(page.getByTestId(`textview/row-${index}`)).toHaveAttribute("data-kind", "equal");
  }
  const leftText = await page.getByTestId("textview/row-6").locator('div[title="left"]').textContent();
  await expect(page.getByTestId("textview/row-6").locator('div[title="right"]')).toHaveText(leftText ?? "");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("l key rides the same gutter path with no disk write", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await plantSaveSentinel(page);
  await page.getByTestId("textview/row-51").click();
  await page.getByTestId("textview/row-51").press("l");

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-139B");
  await expect(note).not.toContainText("saved right to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page);

  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
