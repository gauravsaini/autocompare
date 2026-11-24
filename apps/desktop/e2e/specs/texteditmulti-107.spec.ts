/**
 * Wave-107 multiline cells + draft safety + one-level undo.
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`). Wave-198 staged form: commits
 * stage locally (`edit-note`); Save replays them through
 * `text_apply_edit` (`applied-note`); undo of a staged edit reverts
 * locally with no IPC, undo of an applied edit re-applies via the
 * backend (and can honestly fail).
 *
 * Drives the REAL `App` workspace over the auto-boot demo text diff
 * and proves:
 *  - the RowLine editor is an auto-sized `<textarea>` prefilled with
 *    the cell text, with key hints in its `title` tooltip;
 *  - `Ctrl/Cmd+Enter` stages (single-line and multiline), Save applies;
 *  - plain `Enter` on a still-single-line draft stages (legacy
 *    66ub/66v/67a compat), while `Enter` on an already-multiline
 *    draft inserts a newline;
 *  - blur (clicking away) keeps the draft open; `Esc` cancels;
 *  - `textview/undo-edit` is disabled when empty, restores the
 *    previous text of the most recently committed edit (locally while
 *    staged, via `text_apply_edit` once applied), and reports honestly
 *    on failure.
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

async function firstChangedLeftCell(page: Page) {
  const row = page.locator('.tv-row[data-kind="changed"]').first();
  await expect(row).toBeVisible();
  return { row, cell: row.locator(".tv-cell").first() };
}

function trackErrors(page: Page): string[] {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return pageErrors;
}

test("editor is an auto-sized textarea with key hints", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  expect(before.length).toBeGreaterThan(0);
  await page.screenshot({ path: "/tmp/editmulti-107-before.png" });
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await expect.poll(async () => editor.evaluate((el) => el.tagName)).toBe("TEXTAREA");
  await expect(editor).toHaveValue(before);
  await expect(editor).toHaveAttribute("title", /Ctrl\+Enter/);
  await expect(editor).toHaveAttribute("title", /Esc/);
  await page.screenshot({ path: "/tmp/editmulti-107-editing.png" });

  // Dark scheme: editor holds.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(200);
  await expect(editor).toBeVisible();
  await page.emulateMedia({ colorScheme: "light" });

  await editor.press("Escape");
  await expect(editor).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("Ctrl+Enter stages a single-line edit, Save applies it", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill(`${before} MULTI107-SINGLE`);
  await editor.press("Control+Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(cell).toContainText("MULTI107-SINGLE");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("plain Enter stages a single-line draft (legacy compat), Save applies it", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill(`${before} LEGACY107`);
  await editor.press("Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(cell).toContainText("LEGACY107");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("Enter adds a line once multiline, Ctrl+Enter stages both, Save applies", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const { cell } = await firstChangedLeftCell(page);
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill("LINE-ONE-107\nLINE-TWO-107");
  await editor.press("Enter");
  // Multiline draft: Enter inserts a newline, nothing is staged.
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(1);
  await expect(editor).toHaveValue("LINE-ONE-107\nLINE-TWO-107\n");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await editor.press("Control+Enter");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to");
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(cell).toContainText("LINE-ONE-107");
  await expect(cell).toContainText("LINE-TWO-107");
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("blur keeps the draft open, Esc cancels with zero change", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const { cell } = await firstChangedLeftCell(page);
  const before = (await cell.textContent()) ?? "";
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill("DRAFT-KEPT-107");
  // Click away (the inert status line): blur must NOT cancel.
  await page.getByTestId("textview/status").click();
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(1);
  await expect(editor).toHaveValue("DRAFT-KEPT-107");
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  await editor.press("Escape");
  await expect(page.getByTestId("textview/cell-edit")).toHaveCount(0);
  await expect(cell).toHaveText(before);
  await expect(page.getByTestId("textview/applied-note")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("undo restores the previous text, then disables", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const undo = page.getByTestId("textview/undo-edit");
  await expect(undo).toBeVisible();
  await expect(undo).toBeDisabled();

  const { cell } = await firstChangedLeftCell(page);
  const before = ((await cell.textContent()) ?? "").trim();
  expect(before.length).toBeGreaterThan(0);
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill("UNDO-TARGET-107");
  await editor.press("Control+Enter");
  // Wave-198: the commit is still staged — undo reverts it locally.
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(undo).toBeEnabled();

  await page.screenshot({ path: "/tmp/editmulti-107-before-undo.png" });
  await undo.click();
  await expect(page.getByTestId("textview/undo-note")).toContainText("Undid last edit");
  await expect(undo).toBeDisabled();
  await expect(cell).toHaveText(before);
  await page.screenshot({ path: "/tmp/editmulti-107-after.png" });

  // Dark scheme: receipt + affordances hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(200);
  await expect(page.getByTestId("textview/undo-note")).toBeVisible();
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("failed undo keeps the stack with an honest error", async ({ page }) => {
  const pageErrors = trackErrors(page);
  const undo = page.getByTestId("textview/undo-edit");
  const { cell } = await firstChangedLeftCell(page);
  await cell.dblclick();
  const editor = page.getByTestId("textview/cell-edit");
  await expect(editor).toBeVisible();
  await editor.fill("UNDO-FAIL-107");
  await editor.press("Control+Enter");
  await expect(page.getByTestId("textview/edit-note")).toContainText("1 local edit");
  await expect(undo).toBeEnabled();

  // Wave-198: staged undos are local and cannot fail — flush via Save
  // first so the undo rides the backend path, then break it.
  await page.getByTestId("textview/save-left").click();
  await expect(page.getByTestId("textview/applied-note")).toContainText("1 edit applied");
  await expect(page.getByTestId("textview/edit-note")).toHaveCount(0);

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_APPLY_EDIT_NEXT__ = {
      mode: "failure",
      error: "mock undo failed 107",
    };
  });
  await undo.click();
  await expect(page.getByTestId("textview/edit-error")).toContainText("Undo failed");
  await expect(page.getByTestId("textview/edit-error")).toContainText("mock undo failed 107");
  // Stack kept: retry stays available, applied text untouched.
  await expect(undo).toBeEnabled();
  await expect(cell).toContainText("UNDO-FAIL-107");
  await expect(page.getByTestId("textview/undo-note")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});
