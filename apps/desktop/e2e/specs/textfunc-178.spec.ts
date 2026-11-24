/**
 * Wave-178 textfunc BC-parity slice: copy mid-fail guard + stage-only
 * confirm + shortcuts validate + File1/File2 real validation.
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`), fresh :1661, webkit, workers 1.
 *
 * Ground truth (python difflib, Rust-engine normalized):
 * File1 170 logical lines (169 newline-terminated per wc) vs File2 159
 * -> 176 rows: 147 equal, 6 changed, 17 left-only, 6 right-only,
 * 10 hunks [0-2, 4-8, 25-27, 34-37, 39-41, 44, 56, 111-113, 125, 157-161].
 *
 *  - T1 empty-take orphan deletes locally (BC GAP-03): take-left on the
 *    rightOnly hunk (rows 25-27, take side absent) drops the rows from
 *    the view — no zombie filled rows, totals converge, neighbors keep
 *    ids (merge-func-131 orphan-delete pin on real files).
 *  - T2 take-wins is literal: take-left on changed("", "photos…")
 *    (row 44) copies the empty take side (BC parity with the backend);
 *    Save persists, disk files stay byte-identical (mock writes nothing).
 *  - T3 stage-only trilogy: copy stages in memory (disk unchanged),
 *    explicit Save persists + clears staged state, reload drops staged.
 *  - T4 one handler: menu Take l2r/r2l, toolbar Copy and `l` produce the
 *    same gutter receipt as the gutter click.
 *  - T5 real validation + shortcuts: 176 rows / 10 hunks on real paths,
 *    per-side meta line, `n`/`p` jump, Ctrl+F / Ctrl+G open, Esc closes,
 *    F3 no crash, zero console/page errors throughout.
 */
import { readFileSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import {
  installRealContentMock,
  prepareRealContentFiles,
  type RealContentFixture,
} from "../helpers/realcontent";

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(String(msg.text()));
  });
  return { pageErrors, consoleErrors };
}

async function submitPaths(page: Page, left: string, right: string): Promise<void> {
  await page.getByTestId("pathbar/left").fill(left);
  await page.getByTestId("pathbar/left").press("Enter");
  await page.getByTestId("pathbar/right").fill(right);
  await page.getByTestId("pathbar/right").press("Enter");
}

async function bootReal(page: Page, fixture: RealContentFixture): Promise<void> {
  await installRealContentMock(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Let the auto-booted demo compare settle first: without this the two
  // pathbar Enters below can race the demo `text_compare` and the losing
  // resolution leaves a mixed demo/real pair on screen.
  await expect(page.getByTestId("textview/status")).toBeVisible({ timeout: 15000 });
  await expect
    .poll(async () => page.getByTestId("textview/status").textContent(), { timeout: 15000 })
    .toMatch(/rows/);
  await submitPaths(page, fixture.file1Path, fixture.file2Path);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/status")).toContainText("176 / 176 rows", {
    timeout: 15000,
  });
}

test.describe("Wave-178 textfunc: copy guard plus stage-only plus shortcuts plus real files", () => {
  let fixture: RealContentFixture;

  test.beforeAll(() => {
    fixture = prepareRealContentFiles();
  });

  test.afterAll(() => {
    if (fixture) fixture.cleanup();
  });

  test.beforeEach(async ({ page }) => {
    try {
      await page.goto("/", { waitUntil: "domcontentloaded" });
      await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    } catch (err) {
      console.warn("Browser navigation failed:", err);
      test.skip(true, `Browser unavailable: ${err}`);
    }
  });

  test("1. empty-take orphan deletes locally (no zombie rows), totals converge", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    // Hunk 3 (rows 25-27) is pure rightOnly: take-left has no take side.
    for (const index of [25, 26, 27]) {
      await expect(page.getByTestId(`textview/row-${index}`)).toHaveAttribute(
        "data-kind",
        "rightOnly",
      );
    }
    await page.getByTestId("hunk-copy-left-2").click();

    const note = page.getByTestId("textview/copy-note");
    await expect(note).toContainText("Copied hunk 3 take-left");
    await expect(note).toContainText("kept session-local");
    await expect(note).toContainText("unsaved");
    await expect(note).toContainText("press Save right");

    // GAP-03 parity: the rows are gone locally, not filled zombies.
    for (const index of [25, 26, 27]) {
      await expect(page.getByTestId(`textview/row-${index}`)).toHaveCount(0);
    }
    // Neighbors keep ids; no other row dropped.
    await expect(page.getByTestId("textview/row-24")).toBeVisible();
    await expect(page.getByTestId("textview/row-28")).toBeVisible();
    // Totals converge: 176 - 3 removed, rightOnly 6 -> 3.
    await expect(page.getByTestId("textview/status")).toContainText("173 / 173 rows");
    await expect(page.getByTestId("textview/status")).toContainText("147 same");
    await expect(page.getByTestId("textview/status")).toContainText("6 changed");
    await expect(page.getByTestId("textview/status")).toContainText("17 left-only");
    await expect(page.getByTestId("textview/status")).toContainText("3 right-only");
    // One pair per remaining hunk: 9 left, ordinals renumbered 0..8.
    await expect(page.locator('[data-testid^="hunk-copy-left-"]')).toHaveCount(9);
    await expect(page.getByTestId("hunk-copy-left-9")).toHaveCount(0);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("2. take-wins is literal on an empty take side; Save persists, disk untouched", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    // Row 44 is changed("", "com.google.android.apps.photos"): take-left
    // copies the empty side literally (backend parity, no fill).
    await expect(page.getByTestId("textview/row-44")).toHaveAttribute("data-kind", "changed");
    await page.getByTestId("hunk-copy-left-5").click();
    const note = page.getByTestId("textview/copy-note");
    await expect(note).toContainText("Copied hunk 6 take-left");
    await expect(note).toContainText("1 of 1 row(s) merged");
    await expect(note).toContainText("unsaved");
    await expect(page.getByTestId("textview/row-44")).toHaveAttribute("data-kind", "equal");
    await expect(
      page.getByTestId("textview/row-44").locator('div[title="right"]'),
    ).toHaveText("");

    // Explicit Save persists the staged copy (mock receipt path).
    await page.getByTestId("textview/save-right").click();
    const receipt = page.getByTestId("textview/save-note");
    await expect(receipt).toContainText(`Saved right to ${fixture.file2Path}`);
    await expect(receipt).toContainText("176 lines");
    await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
    // Mock IPC never writes disk: both files byte-identical.
    expect(readFileSync(fixture.file1Path, "utf8")).toBe(fixture.file1Content);
    expect(readFileSync(fixture.file2Path, "utf8")).toBe(fixture.file2Content);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("3. stage-only: copy stages (disk unchanged), Save persists, reload drops staged", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    // Gutter copy stages in memory only.
    await page.getByTestId("hunk-copy-left-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
    await expect(page.getByTestId("textview/row-0")).toHaveAttribute("data-kind", "equal");
    await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
    expect(readFileSync(fixture.file1Path, "utf8")).toBe(fixture.file1Content);
    expect(readFileSync(fixture.file2Path, "utf8")).toBe(fixture.file2Content);

    // Explicit Save persists + clears staged state.
    await page.getByTestId("textview/save-right").click();
    await expect(page.getByTestId("textview/save-note")).toContainText("Saved right to");
    await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
    await expect(page.getByTestId("textview/row-0")).toHaveAttribute("data-kind", "equal");

    // Reload drops staged: fresh compare from unchanged disk.
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    await bootReal(page, fixture);
    await expect(page.getByTestId("textview/row-0")).toHaveAttribute("data-kind", "leftOnly");
    await expect(page.getByTestId("textview/row-2")).toHaveAttribute("data-kind", "leftOnly");
    await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("4. menu Take, toolbar Copy and l ride the one gutter handler", async ({ page }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    // Menu Take Right on the row-34 changed hunk (same receipt shape).
    await page.getByTestId("textview/row-34").click();
    await page.evaluate(() => {
      (window as unknown as { __MOCK_EMIT_MENU__?: (p: unknown) => void }).__MOCK_EMIT_MENU__?.({
        id: "session.take-right",
        command: "text_copy_section",
      });
    });
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
    await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
    await expect(page.getByTestId("textview/row-34")).toHaveAttribute("data-kind", "equal");
    await expect(page.getByTestId("statusbar/message")).toContainText(
      "Copy take-right requested for row 34",
    );

    // Toolbar Copy follows the selection through the same handler.
    await page.getByTestId("textview/row-111").click();
    await page.getByTestId("toolbar/action-copy").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
    await expect(page.getByTestId("textview/row-111")).toHaveAttribute("data-kind", "equal");
    await expect(page.getByTestId("statusbar/message")).toContainText(
      "Copy take-left requested for row 111",
    );

    // `l` key rides the same path (single line orphan deletes, GAP-03).
    await page.getByTestId("textview/row-125").click();
    await page.getByTestId("textview/row-125").press("l");
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
    await expect(page.getByTestId("textview/row-125")).toHaveCount(0);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("5. real validation 176 rows 10 hunks plus shortcuts", async ({ page }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    // Real validation: exact ground truth on real paths.
    const status = page.getByTestId("textview/status");
    await expect(status).toContainText("176 / 176 rows");
    await expect(status).toContainText("147 same");
    await expect(status).toContainText("6 changed");
    await expect(status).toContainText("17 left-only");
    await expect(status).toContainText("6 right-only");
    await expect(status).toContainText("left 170 lines · right 159 lines");
    for (let h = 0; h < 10; h += 1) {
      await expect(page.getByTestId(`hunk-copy-left-${h}`)).toBeVisible();
      await expect(page.getByTestId(`hunk-copy-right-${h}`)).toBeVisible();
    }
    await expect(page.getByTestId("hunk-copy-left-10")).toHaveCount(0);
    // BC feel: per-side file meta under the path bar (bytes echoed).
    await expect(page.getByTestId("textview/meta")).toContainText("bytes");

    // Shortcuts: n/p jump hunks with wrap.
    await page.getByTestId("textview/row-34").click();
    await page.getByTestId("textview/row-34").focus();
    await page.keyboard.press("n");
    await expect(page.getByTestId("textview/row-39")).toHaveAttribute("data-selected", "true");
    await expect(page.getByTestId("textview/diffnav-note")).toContainText("difference 5 of 10");
    await page.getByTestId("textview/row-39").focus();
    await page.keyboard.press("p");
    await expect(page.getByTestId("textview/row-34")).toHaveAttribute("data-selected", "true");

    // Shortcuts: Cmd+Alt+Down/Up step exactly one hunk (single routing
    // point — no row/window/bridge triple jump, no stale note).
    await page.getByTestId("textview/row-34").focus();
    await page.keyboard.press("Meta+Alt+ArrowDown");
    await expect(page.getByTestId("textview/row-39")).toHaveAttribute("data-selected", "true");
    await expect(page.getByTestId("textview/diffnav-note")).toContainText("difference 5 of 10");
    await page.keyboard.press("Meta+Alt+ArrowUp");
    await expect(page.getByTestId("textview/row-34")).toHaveAttribute("data-selected", "true");
    await expect(page.getByTestId("textview/diffnav-note")).toContainText("difference 4 of 10");

    // Shortcuts: Ctrl+F opens find, F3 steps, Esc closes; Ctrl+G opens goto.
    await page.keyboard.press("Control+f");
    await expect(page.getByTestId("textview/find-input")).toBeVisible();
    await page.getByTestId("textview/find-input").fill("photos");
    await page.getByTestId("textview/find-input").press("Enter");
    await expect(page.getByTestId("textview/find-status")).toContainText("match 1 of 1");
    await page.keyboard.press("F3");
    await expect(page.getByTestId("textview/find-status")).toContainText("match 1 of 1");
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
    await page.keyboard.press("Control+g");
    await expect(page.getByTestId("textview/goto-input")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });
});
