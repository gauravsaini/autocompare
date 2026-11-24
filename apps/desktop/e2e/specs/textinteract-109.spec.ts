/**
 * Wave-109 keyboard + caret affordance polish (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-boot demo text diff
 * (real engine rows) and proves interaction affordance only:
 *  - rows keep single-click select (no editor) + dblclick/Enter-to-edit;
 *  - arrows move selection; `n` / `p` jump hunks with wrap and honest
 *    position; `l` / `r` take left/right via keyboard; `Escape` on a
 *    row clears the selection;
 *  - the take arrows stay keyboard-operable (focus + Enter resolves);
 *  - editor `Esc` cancels with zero change, restores cell focus, and
 *    does NOT collapse the find/goto panels;
 *  - commit restores cell focus; undo restores row focus;
 *  - focus-visible rings ride keyboard focus (light + dark).
 *
 * New file only, no existing spec touched. Skip policy matches the
 * other specs.
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
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  return { pageErrors, consoleErrors };
}

function expectZeroNoise(pageErrors: string[], consoleErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

async function firstChangedRow(page: Page) {
  const row = page.locator('.tv-row[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  return row;
}

async function rowIndex(row: import("@playwright/test").Locator): Promise<number> {
  const testid = (await row.getAttribute("data-testid")) ?? "";
  return Number(testid.replace("textview/row-", ""));
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

async function changedCount(page: Page): Promise<number> {
  return page.locator('.tv-row[data-kind="changed"]').count();
}

test("keyboard: arrows move selection, n/p jump hunks with wrap + honest position", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const row = await firstChangedRow(page);
  const idx = await rowIndex(row);
  await row.click();
  await expect(row).toHaveAttribute("data-selected", "true");
  await expect.poll(() => focusedTestId(page)).toBe(`textview/row-${idx}`);

  // ArrowDown moves selection exactly one loaded row down, keeping focus.
  await page.keyboard.press("ArrowDown");
  const downIdx = idx + 1;
  await expect(page.getByTestId(`textview/row-${downIdx}`)).toHaveAttribute("data-selected", "true");
  await expect.poll(() => focusedTestId(page)).toBe(`textview/row-${downIdx}`);
  await expect(page.locator('[data-testid^="textview/row-"][data-selected="true"]')).toHaveCount(1);

  // ArrowUp returns.
  await page.keyboard.press("ArrowUp");
  await expect(page.getByTestId(`textview/row-${idx}`)).toHaveAttribute("data-selected", "true");
  await expect.poll(() => focusedTestId(page)).toBe(`textview/row-${idx}`);

  // n / p walk hunks relative to the selection (the demo's first
  // `changed` row sits mid-list: hunk 0/1 are one-sided). Parse the
  // honest position, then prove step +1 / -1 / full-cycle wrap.
  await page.keyboard.press("n");
  const note = page.getByTestId("textview/diffnav-note");
  await expect(note).toContainText(/difference \d+ of \d+/);
  const m1 = (await note.innerText()).match(/difference (\d+) of (\d+)/);
  expect(m1).not.toBeNull();
  const at = Number(m1?.[1]);
  const total = Number(m1?.[2]);
  expect(at).toBeGreaterThanOrEqual(1);
  expect(total).toBeGreaterThanOrEqual(2);
  const next = (at % total) + 1;
  await expect(page.getByTestId("textview/toolbar-hunks")).toContainText(new RegExp(`hunk ${at} of ${total}`));
  await expect.poll(() => focusedTestId(page)).toMatch(/^textview\/row-/);
  await expect(page.locator('[data-testid^="textview/row-"][data-selected="true"]')).toHaveCount(1);
  await page.keyboard.press("n");
  await expect(note).toContainText(`difference ${next} of ${total}`);
  await page.keyboard.press("p");
  await expect(note).toContainText(`difference ${at} of ${total}`);
  // Full cycle wraps back to the same hunk (wrap at both ends).
  for (let i = 0; i < total; i += 1) {
    const cur = (await note.innerText()).match(/difference (\d+) of (\d+)/);
    const want = (Number(cur?.[1]) % total) + 1;
    await page.keyboard.press("n");
    await expect(note).toContainText(`difference ${want} of ${total}`);
  }
  await expect(note).toContainText(`difference ${at} of ${total}`);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Dark scheme: nav state holds.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(200);
  await expect(page.getByTestId("textview/diffnav-note")).toContainText(`difference ${at} of ${total}`);
  await page.emulateMedia({ colorScheme: "light" });

  expectZeroNoise(pageErrors, consoleErrors);
});

test("keyboard take: l takes left, focused take button + Enter takes right", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const before = await changedCount(page);
  expect(before).toBeGreaterThan(0);

  // "l" on a hunk row copies left over the hunk: changed rows shrink,
  // the edited row reads equal.
  const row = await firstChangedRow(page);
  const idx = await rowIndex(row);
  await row.click();
  await expect(row).toBeFocused();
  await row.press("l");
  await expect.poll(() => changedCount(page)).toBeLessThan(before);
  await expect(page.getByTestId(`textview/row-${idx}`)).toHaveAttribute("data-kind", "equal");
  await page.screenshot({ path: "/tmp/interact-109-after-take.png" });

  // The take arrows stay keyboard-operable: focus + Enter takes hunk 0
  // (left-only rows 6-9: take-right has an EMPTY take side, so BC
  // parity deletes the orphan lines — Rust `apply_hunk_to_rows`
  // FIXTEXT GAP-03 `apply_hunk_empty_take_deletes_orphan_lines`, the
  // mock `text_apply_hunk`, and the view `applyHunkCopy` all drop
  // null-take rows. Rows 6-9 unmount, the take-button pairs shrink by
  // one, and no orphan tint lingers).
  const takeRight = page.getByTestId("hunk-copy-right-0");
  await expect(takeRight).toBeVisible();
  const pairsBefore = await page.locator('[data-testid^="hunk-copy-left-"]').count();
  expect(pairsBefore).toBeGreaterThan(0);
  await takeRight.focus();
  await expect(takeRight).toBeFocused();
  await takeRight.press("Enter");
  await expect(page.getByTestId("textview/row-6")).toHaveCount(0);
  await expect(page.getByTestId("textview/row-9")).toHaveCount(0);
  await expect
    .poll(() => page.locator('[data-testid^="hunk-copy-left-"]').count())
    .toBe(pairsBefore - 1);

  await expect(page.getByRole("alert")).toHaveCount(0);
  expectZeroNoise(pageErrors, consoleErrors);
});

test("caret: single click selects without editing, Esc cancels + restores cell focus + keeps panels", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);
  // Wave-112 preamble (batch-green extension, same pattern as the five
  // named specs): find/goto panels are closed by default; open them so
  // the editor-Esc below proves it keeps panels open. Zero assertion
  // changes.
  for (const toggle of ["textview/toolbar-find", "textview/toolbar-goto"] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded !== "true") await page.getByTestId(toggle).click();
  }

  const row = await firstChangedRow(page);
  const cell = row.locator(".tv-cell").first();
  const before = (await cell.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);

  // Single click selects, never edits.
  await cell.click();
  await expect(row).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);

  // Dblclick edits; Esc cancels with zero change and focus back on the cell.
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill("SHOULD-NOT-STICK-109");
  await page.screenshot({ path: "/tmp/interact-109-editing.png" });
  await editor.press("Escape");
  await expect(editor).toHaveCount(0);
  await expect(cell).toHaveText(before);
  await expect(cell).toBeFocused();
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);

  // Esc cancelled the edit ONLY: the find/goto panels stay open.
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();

  // Escape on the row clears the selection (Esc always exits).
  await row.press("Escape");
  await expect.poll(() => page.locator('[data-testid^="textview/row-"][data-selected="true"]').count()).toBe(0);

  expectZeroNoise(pageErrors, consoleErrors);
});

test("commit restores cell focus, undo restores row focus", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const row = await firstChangedRow(page);
  const idx = await rowIndex(row);
  const cell = row.locator(".tv-cell").first();
  const before = ((await cell.textContent()) ?? "").trim();

  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill("FOCUS-RESTORE-109");
  await editor.press("Control+Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(cell).toContainText("FOCUS-RESTORE-109");
  // Caret affordance: keyboard flow continues in the edited cell.
  await expect(cell).toBeFocused();

  const undo = page.getByTestId("textview/undo-edit");
  await expect(undo).toBeEnabled();
  await undo.click();
  await expect(page.getByTestId("textview/undo-note")).toContainText("Undid last edit");
  await expect(undo).toBeDisabled();
  await expect(cell).toHaveText(before);
  // Undo hands focus back to the restored row.
  await expect.poll(() => focusedTestId(page)).toBe(`textview/row-${idx}`);

  // Dark scheme: receipt + restored row hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(200);
  await expect(page.getByTestId("textview/undo-note")).toBeVisible();
  await page.screenshot({ path: "/tmp/interact-109-undo-dark.png" });
  await page.emulateMedia({ colorScheme: "light" });

  expectZeroNoise(pageErrors, consoleErrors);
});

test("focus-visible rings ride keyboard focus (light + dark)", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Tab from the keyboard until a text row holds focus, then prove the
  // shared :focus-visible ring actually paints (2px solid, not none).
  let guard = 0;
  while (guard < 120) {
    const id = await focusedTestId(page);
    if (/^textview\/row-\d+$/.test(id)) break;
    await page.keyboard.press("Tab");
    guard += 1;
  }
  const focusedId = await focusedTestId(page);
  expect(focusedId, "a text row is reachable by keyboard Tab").toMatch(/^textview\/row-\d+$/);
  const ring = await page.evaluate(() => {
    const el = document.activeElement;
    if (!(el instanceof HTMLElement)) return { matches: false, width: "", style: "" };
    const cs = getComputedStyle(el);
    return { matches: el.matches(":focus-visible"), width: cs.outlineWidth, style: cs.outlineStyle };
  });
  expect(ring.matches, "keyboard-focused row matches :focus-visible").toBe(true);
  expect(ring.width, "ring paints 2px").toBe("2px");
  expect(ring.style, "ring paints solid").toBe("solid");
  await page.screenshot({ path: "/tmp/interact-109-focus-light.png" });

  // Dark scheme: next Tab stop keeps a visible ring too.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(200);
  await page.keyboard.press("Tab");
  const darkRing = await page.evaluate(() => {
    const el = document.activeElement;
    if (!(el instanceof HTMLElement)) return { matches: false, width: "", style: "" };
    const cs = getComputedStyle(el);
    return { matches: el.matches(":focus-visible"), width: cs.outlineWidth, style: cs.outlineStyle };
  });
  expect(darkRing.matches, "dark-scheme keyboard focus matches :focus-visible").toBe(true);
  expect(darkRing.width, "dark-scheme ring paints 2px").toBe("2px");
  await page.screenshot({ path: "/tmp/interact-109-focus-dark.png" });
  await page.emulateMedia({ colorScheme: "light" });

  expectZeroNoise(pageErrors, consoleErrors);
});
