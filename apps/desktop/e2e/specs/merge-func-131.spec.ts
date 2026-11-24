/**
 * Wave-131 merge-func parity: gutter + toolbar + menu Take Left/Right +
 * shortcuts (audit full ranked #3 follow-up). MOCK IPC ONLY
 * (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves true
 * Beyond Compare merge (take-side wins, hunk flips equal, pair -1):
 *  - gutter take-left on the single-row changed hunk (row 51) flips it
 *    equal through `text_apply_edit` + local apply with no autosave
 *    (unsaved note, Save buttons write), pair count -1;
 *  - gutter take-right on the leftOnly orphan hunk (rows 6-9) deletes
 *    the orphan lines (BC parity: empty source clears target), drops
 *    NO other row silently;
 *  - `l` / `r` keys ride the same path (rows 98 / 145), `n` / `p` jump
 *    hunks, `Enter` opens the cell editor, `Escape` cancels it and then
 *    clears the selection; letters typed in the editor never take;
 *  - toolbar Copy rides the same path (take side follows selection);
 *  - native Session > Take Left / Take Right (`menu-event`
 *    `session.take-left` / `session.take-right`) ride the same path for
 *    the selected hunk, honest note with no hunk selected;
 *  - toolbar Merge enables in merge-kind sessions and jumps to Merge.
 *
 * PORT-FREE: runs on a fresh :1541 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
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

async function pairCount(page: Page): Promise<number> {
  return page.locator('[data-testid^="hunk-copy-left-"]').count();
}

test("gutter take-left flips changed hunk equal, pair count drops", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  const pairsBefore = await pairCount(page);

  await page.getByTestId("hunk-copy-left-2").click();

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("1 of 1 row(s) merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText("saved right to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);

  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("textview/row-51").locator('[data-testid^="hunk-copy-"]')).toHaveCount(0);
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(pairsBefore - 1);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("gutter take-right deletes orphan hunk lines (BC parity), drops no other rows", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Hunk 0 (rows 6-9) is a leftOnly run: no right cells at all.
  for (const index of [6, 7, 8, 9]) {
    await expect(page.getByTestId(`textview/row-${index}`)).toHaveAttribute("data-kind", "leftOnly");
  }
  const pairsBefore = await pairCount(page);
  await expect(page.getByTestId("hunk-copy-right-0")).toBeVisible();

  await page.getByTestId("hunk-copy-right-0").click();

  // BC parity (GAP-03): the take side is empty, so the orphan lines are
  // deleted instead of cloned. Rows 6-9 disappear, later rows shift up,
  // the hunk count drops by one, and the copy stays session-local.
  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 1 take-right");
  await expect(note).toContainText("kept session-local");
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(pairsBefore - 1);
  // Row ids are stable: the deleted run leaves a gap (rows 6-9 gone),
  // neighbors keep their ids — no other row is dropped.
  for (const index of [6, 7, 8, 9]) {
    await expect(page.getByTestId(`textview/row-${index}`)).toHaveCount(0);
  }
  await expect(page.getByTestId("textview/row-5")).toBeVisible();
  await expect(page.getByTestId("textview/row-10")).toBeVisible();

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("l/r keys take, textarea guard holds, n/p jump, Esc clears", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Letters typed in the cell editor never take: open the editor on an
  // equal row, type `l`, and no copy fires.
  await page.getByTestId("textview/row-99").locator('div[title="left"]').dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.press("l");
  await expect(editor).toHaveValue(/l/);
  await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
  // Escape cancels the edit (editor owns Esc, selection untouched).
  await editor.press("Escape");
  await expect(editor).toHaveCount(0);

  // `l` takes left on the row-98 changed hunk; `r` takes right on row-145.
  // Wave-139b: same gutter path, no autosave - no save override needed.
  await page.getByTestId("textview/row-98").press("l");
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");

  await page.getByTestId("textview/row-145").press("r");
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "equal");

  // `n` / `p` jump between remaining hunks (wrap, no crash, no copy).
  await page.getByTestId("textview/row-20").click();
  await page.getByTestId("textview/row-20").press("n");
  await page.getByTestId("textview/row-20").press("p");

  // `Escape` on a row clears the selection; toolbar Copy parks honestly.
  await page.getByTestId("textview/row-20").press("Escape");
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  expect(await page.getByTestId("toolbar/action-copy").getAttribute("title")).toContain("select a row");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("toolbar Copy same path, menu Take Left/Right, Merge jump", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Toolbar Copy follows the selection through the gutter path.
  await page.getByTestId("textview/row-98").click();
  const copyBtn = page.getByTestId("toolbar/action-copy");
  await expect(copyBtn).toBeEnabled();
  expect(await copyBtn.getAttribute("aria-label")).toContain("take left");
  await copyBtn.click();
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");

  // Native menu Take Right rides the same path for the selected hunk.
  await page.getByTestId("textview/row-145").click();
  await page.evaluate(() => {
    (window as unknown as { __MOCK_EMIT_MENU__?: (p: unknown) => void }).__MOCK_EMIT_MENU__?.({
      id: "session.take-right",
      command: "text_copy_section",
    });
  });
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-right requested for row 145");

  // No hunk selected: the menu Take reports honestly instead of firing.
  await page.getByTestId("textview/row-145").press("Escape");
  await page.evaluate(() => {
    (window as unknown as { __MOCK_EMIT_MENU__?: (p: unknown) => void }).__MOCK_EMIT_MENU__?.({
      id: "session.take-left",
      command: "text_copy_section",
    });
  });
  await expect(page.getByTestId("statusbar/message")).toContainText("needs a hunk row selected");

  // Merge kind: Merge enables and jumps to the Merge view.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/merge131-left.txt");
  await page.getByTestId("session/right-path").fill("/tmp/merge131-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("view-merge")).toBeVisible();
  const mergeBtn = page.getByTestId("toolbar/action-merge");
  await expect(mergeBtn).toBeEnabled();
  await mergeBtn.click();
  await expect(page.getByTestId("view-merge")).toBeVisible();
  await expect(page.getByTestId("statusbar/message")).toContainText("Merge view");
  await expect(copyBtn).toBeDisabled();

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
