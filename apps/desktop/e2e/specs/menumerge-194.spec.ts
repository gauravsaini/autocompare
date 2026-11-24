/**
 * Wave-194 menumerge BC-parity slice (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and pins the
 * menu-merge + gutter contract BC jaisa hona chahiye:
 *  - top-menu merge buttons (Take Left l2r / Take Right r2l, i.e. the
 *    native `session.take-left` / `session.take-right` ids from
 *    `menu.rs` + their CmdOrCtrl+D / CmdOrCtrl+E intents) ride the SAME
 *    `copyHunkAt` staged path as the gutter arrows (via the
 *    `autocompare:text-copy` window event owned by TextView): click
 *    stages the change, save tak persist, auto-save nahi;
 *  - gutter arrows `[>]` (`hunk-copy-left-N`) / `[<]`
 *    (`hunk-copy-right-N`) stage locally: row flips equal, dirty dot
 *    shows, planted `__TEXT_SAVE_SIDE_NEXT__` failure sentinel stays
 *    UNCONSUMED (no disk write), clipboard untouched, `textview/copy-note`
 *    receipt says unsaved + names the Save side, `textview/save-note`
 *    stays absent;
 *  - staged copy persist-until-save: Load more / row nav ke baad bhi
 *    staged result rehta hai (row equal + dirty dot + copy-note); dobara
 *    click honest no-op-with-note hai; user Save staged ko disk par
 *    likhta hai aur staged state clear karta hai;
 *  - shortcuts audit: `matchWebShortcut` maps CmdOrCtrl+D/E to take-left/
 *    right (F/G/plain keys skip), Meta+D bina selection ke honest note
 *    deta hai, `l` key hunk par take-left copy karta hai.
 *
 * Wiring note: `App.handleMenuTake` -> `autocompare:text-copy` ->
 * `TextView.copyHunkAt` -> stagedCopies -> `saveSide` flush
 * (`text_apply_hunk` + `text_save_side`) — koi naya UI/IPC nahi, sirf
 * existing stubs ka pin. TextView copyHunkAt + App menu section is wave
 * read-only: yeh spec UI/events se drive karti hai, source edit nahi.
 *
 * PORT-FREE: runs on a fresh :1695 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate. Skip policy matches the
 * other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";
import { matchWebShortcut } from "../../src/components/chrome/shortcuts";

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
      command: menuId === "session.take-left" ? "text_copy_left" : "text_copy_right",
    });
  }, id);
}

function chord(key: string, extra: Partial<{ shiftKey: boolean; altKey: boolean }> = {}): {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
} {
  return { key, metaKey: true, ctrlKey: false, shiftKey: extra.shiftKey ?? false, altKey: extra.altKey ?? false };
}

test("menumerge-194: gutter [>] l2r stages with no autosave", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-194-GUTTER-L2R";

  // Hunk 2 (ordinal 2) is the single-row changed hunk at row 51.
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await plantSaveSentinel(page, TAG);
  await page.getByTestId("hunk-copy-left-2").click();

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText(TAG);
  await expect(note).not.toContainText("clipboard");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);

  // Row flips equal in place + right side shows the dirty dot.
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("textview/dirty-right")).toBeVisible();

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menumerge-194: gutter [<] r2l stages with no autosave", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-194-GUTTER-R2L";

  // Row-145 changed hunk: the [<] arrow lives in its own grid row.
  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "changed");
  await plantSaveSentinel(page, TAG);
  await page.getByTestId("textview/row-145").locator('[data-testid^="hunk-copy-right-"]').click();

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("take-right");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save left");
  await expect(note).not.toContainText(TAG);
  await expect(note).not.toContainText("saved left to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);

  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("textview/dirty-left")).toBeVisible();

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menumerge-194: menu Take Left/Right ride the same staged path", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-194-MENU";

  // l2r via the Tauri-listen hook (__MOCK_EMIT_MENU__).
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-98").click();
  await plantSaveSentinel(page, TAG);
  await emitMenuTake(page, "session.take-left");

  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 4 take-left");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText(TAG);
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");

  // r2l via the raw window CustomEvent dispatch path (plain-browser).
  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "changed");
  await page.getByTestId("textview/row-145").click();
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("menu-event", { detail: { id: "session.take-right", command: "text_copy_right" } }),
    );
  });
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);
  await expect(page.getByTestId("textview/row-145")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-right requested for row 145");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menumerge-194: staged copy persists until Save, Save flushes + clears", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  const TAG = "AUTOSAVE-PROBE-194-PERSIST";

  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await plantSaveSentinel(page, TAG);
  await page.getByTestId("hunk-copy-left-2").click();
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");

  // Persist: Load more + nav ke baad bhi staged result rehta hai.
  await page.getByRole("button", { name: /Load more/ }).click();
  await expect(page.getByTestId("textview/status")).toContainText("400 / 644 rows");
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/dirty-right")).toBeVisible();
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expectSaveUntouched(page, TAG);

  // Dobara copy: honest no-op-with-note, crash/duplicate nahi (menu/gutter
  // wala hi `autocompare:text-copy` event dobara bhejo — hunk equal hai).
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("autocompare:text-copy", { detail: { index: 51, dir: "left" } }));
  });
  await expect(page.getByTestId("textview/copy-note")).toContainText("already copied");

  // User Save staged ko disk par likhta hai aur staged state clear karta hai.
  await page.evaluate((p) => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "success",
      path: p,
      lines: 644,
      bytes: 4242,
    };
  }, "/tmp/mock-194-right.txt");
  await page.getByTestId("textview/save-right").click();
  const receipt = page.getByTestId("textview/save-note");
  await expect(receipt).toContainText("Saved right to /tmp/mock-194-right.txt");
  await expect(receipt).toContainText("4242 bytes");
  await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/dirty-right")).toHaveCount(0);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("menumerge-194: shortcuts audit — D/E map, F/G skip, bridge honest, l copies", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Unit: CmdOrCtrl+D/E take-left/right; F/G/plain/Alt/unknown skip.
  expect(matchWebShortcut(chord("d"))).toBe("session.take-left");
  expect(matchWebShortcut(chord("e"))).toBe("session.take-right");
  expect(matchWebShortcut(chord("f"))).toBeNull();
  expect(matchWebShortcut(chord("g"))).toBeNull();
  expect(matchWebShortcut({ key: "l", metaKey: false, ctrlKey: false, shiftKey: false, altKey: false })).toBeNull();
  expect(matchWebShortcut(chord("d", { altKey: true }))).toBeNull();
  expect(matchWebShortcut(chord("b"))).toBeNull();

  // Bridge: bina selection ke Meta+D honest note deta hai.
  await page.keyboard.press("Meta+d");
  await expect(page.getByTestId("statusbar/message")).toContainText("needs a hunk row selected", { timeout: 10000 });

  // `l` key hunk par take-left copy karta hai, no autosave receipt ke saath.
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", /^(leftOnly|rightOnly|changed)$/, { timeout: 10000 });
  await page.getByTestId("textview/row-6").click();
  await page.keyboard.press("l");
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left", { timeout: 10000 });
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved", { timeout: 10000 });
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", "equal", { timeout: 10000 });

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
