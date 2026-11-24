/**
 * Wave-141 menu Take Left/Right post-139b proof (BC parity). MOCK IPC
 * ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves menu
 * Take Left / Take Right (+ their CmdOrCtrl+D / CmdOrCtrl+E native
 * intents) plus toolbar Copy all ride the SAME wave-139b no-autosave
 * copy path (`copyHunkAt` via the `autocompare:text-copy` window
 * event) with dirty-buffer semantics:
 *  - toolbar Copy on the row-98 changed hunk merges with NO disk write
 *    (planted `__TEXT_SAVE_SIDE_NEXT__` failure sentinel stays
 *    unconsumed, never surfaces) and NO clipboard touch; the
 *    `textview/copy-note` receipt says unsaved and names the Save side;
 *    `textview/save-note` stays absent until the user presses Save;
 *  - native menu Take Right (`menu-event` `session.take-right`, the
 *    exact payload `menu.rs route_menu_event` emits for both the menu
 *    item and its CmdOrCtrl+E accelerator) rides the same path on the
 *    row-145 changed hunk with the same dirty receipt;
 *  - native menu Take Left (`session.take-left`, CmdOrCtrl+D intent)
 *    mirrors on the row-51 single-row hunk, dispatched once via the
 *    `__MOCK_EMIT_MENU__` hook and once via a raw `menu-event`
 *    CustomEvent so both App listeners are proven;
 *  - no hunk selected: the menu Take reports honestly
 *    (`needs a hunk row selected`) instead of firing;
 *  - `r` key mirrors the same path and the user Save afterwards writes
 *    the dirty buffer (forced success receipt), closing the
 *    dirty-buffer loop.
 *
 * Native-accelerator note: CmdOrCtrl+D / CmdOrCtrl+E are Tauri menu
 * accelerators (`menu.rs` `build_menu`, unit-tested by
 * `route_menu_event_take_left_right`); under the mock browser there is
 * no Tauri shell, so the spec dispatches the identical `menu-event`
 * payload the shell would emit and asserts the identical receipt.
 *
 * PORT-FREE: runs on a fresh :1556 server
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

/** Plant a save override that screams if any copy path ever saves. */
async function plantSaveSentinel(page: Page, tag: string): Promise<void> {
  await page.evaluate((t) => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "failure",
      error: t,
    };
  }, tag);
}

/** Every copy path must leave the planted save override unconsumed. */
async function expectSaveUntouched(page: Page, tag: string): Promise<void> {
  const leftover = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__,
  );
  expect((leftover as { error?: unknown } | null)?.error).toBe(tag);
}

async function emitMenuTake(page: Page, id: "session.take-left" | "session.take-right"): Promise<void> {
  await page.evaluate((menuId) => {
    (window as unknown as { __MOCK_EMIT_MENU__?: (p: unknown) => void }).__MOCK_EMIT_MENU__?.({
      id: menuId,
      command: "text_copy_section",
    });
  }, id);
}

test("toolbar Copy rides the 139b no-autosave path (dirty buffer)", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-141-TOOLBAR";

  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-98").click();
  const copyBtn = page.getByTestId("toolbar/action-copy");
  await expect(copyBtn).toBeEnabled();
  expect(await copyBtn.getAttribute("aria-label")).toContain("take left");

  await plantSaveSentinel(page, TAG);
  await copyBtn.click();

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 4 take-left");
  await expect(note).toContainText("1 of 1 row(s) merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText(TAG);
  await expect(note).not.toContainText("saved right to");
  await expect(note).not.toContainText("clipboard");
  await expect(note).not.toContainText("Clipboard");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);

  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menu Take Right (Cmd+E intent) rides the same path with no autosave", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-141-MENU-R";

  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-145").click();

  await plantSaveSentinel(page, TAG);
  await emitMenuTake(page, "session.take-right");

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("take-right");
  await expect(note).toContainText("merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save left");
  await expect(note).not.toContainText(TAG);
  await expect(note).not.toContainText("saved left to");
  await expect(note).not.toContainText("clipboard");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);

  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-right requested for row 145");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menu Take Left (Cmd+D intent) via both hooks; honest with no selection", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-141-MENU-L";

  // Hook 1: the __MOCK_EMIT_MENU__ entry (Tauri listen() equivalent).
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-51").click();
  await plantSaveSentinel(page, TAG);
  await emitMenuTake(page, "session.take-left");

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText(TAG);
  await expect(note).not.toContainText("saved right to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 51");

  // Hook 2: the raw window CustomEvent (plain-browser dispatch path).
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-98").click();
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("menu-event", { detail: { id: "session.take-left", command: "text_copy_section" } }),
    );
  });
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");

  // No hunk selected: honest note, no copy fires.
  await page.getByTestId("textview/row-98").press("Escape");
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  await emitMenuTake(page, "session.take-right");
  await expect(page.getByTestId("statusbar/message")).toContainText("needs a hunk row selected");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("r key mirrors the path; user Save writes the dirty buffer", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-141-RKEY";

  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "changed");
  await plantSaveSentinel(page, TAG);
  await page.getByTestId("textview/row-145").click();
  await page.getByTestId("textview/row-145").press("r");

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("take-right");
  await expect(note).toContainText("unsaved");
  await expect(note).not.toContainText(TAG);
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);
  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "equal");

  // Dirty-buffer close: the user Save writes now (forced success).
  await page.evaluate((p) => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "success",
      path: p,
      lines: 644,
      bytes: 4242,
    };
  }, "/tmp/mock-141-left.txt");
  await page.getByTestId("textview/save-left").click();
  const receipt = page.getByTestId("textview/save-note");
  await expect(receipt).toContainText("Saved left to /tmp/mock-141-left.txt");
  await expect(receipt).toContainText("4242 bytes");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
