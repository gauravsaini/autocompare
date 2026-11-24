/**
 * Wave-125 gutter copy BC persist + toolbar Copy/Merge live (audit H2/H3
 * ranked #3), wave-139b re-pinned to BC-faithful no-autosave semantics.
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves:
 *  - gutter: clicking `hunk-copy-left-2` (single-row changed hunk at row
 *    51) copies left to right through `text_apply_edit` + local apply
 *    with NO autosave (planted save probe stays unconsumed) and NO
 *    clipboard touch, shows the unsaved `textview/copy-note` receipt,
 *    and records NOTHING in localStorage (wave-172b BC-faithful
 *    stage-only: reload clears staged copies, BC wins);
 *  - reload drops the staged result: after `page.reload()` row 51 is
 *    changed again with no copy-note (wave-172b honest update — the
 *    old wave-125 "reload keeps the copy" expectation was removed
 *    because BC parity says unconfirmed merges do not survive
 *    reload; see checks/textmerge-172b/REPORT.md);
 *  - toolbar Copy enables when a hunk row is selected (take side
 *    follows the selection), runs the same path, and disables honestly
 *    on equal rows (`selected row is not a difference`) and with no
 *    selection (`select a row first`);
 *  - toolbar Merge enables in merge-kind sessions and jumps to the
 *    Merge view (`view-merge` visible + status message), parked
 *    otherwise.
 *
 * PORT-FREE: runs on a fresh :1533 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
 *
 * Skip policy matches the other specs (infra failures skip).
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
  await page.getByRole("button", { name: /Load more/ }).click();
  await expect(page.getByTestId("textview/status")).toContainText("400 / 644 rows");
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

async function storedCopyRecord(page: Page): Promise<string | null> {
  return page.evaluate(() => {
    const keys: string[] = [];
    for (let i = 0; i < window.localStorage.length; i += 1) {
      const k = window.localStorage.key(i);
      if (k !== null && k.startsWith("ac:textcopy125:")) keys.push(k);
    }
    if (keys.length === 0) return null;
    return window.localStorage.getItem(keys[0] as string);
  });
}

test("gutter copy merges with no autosave and reload drops it (BC stage-only)", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Hunk 2 (ordinal 2) is the single-row changed hunk at row 51.
  const rightOf51 = page.getByTestId("textview/row-51").locator('div[title="right"]');
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed");
  await expect(rightOf51).toContainText("right variant");
  await expect(page.getByTestId("hunk-copy-left-2")).toBeVisible();
  const pairCount = async (): Promise<number> =>
    page.locator('[data-testid^="hunk-copy-left-"]').count();
  const pairsBefore = await pairCount();

  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__ = {
      mode: "failure",
      error: "AUTOSAVE-PROBE-125",
    };
  });
  await page.getByTestId("hunk-copy-left-2").click();

  // Receipt (wave-139b BC-faithful): hunk + take side + merged +
  // unsaved. Gutter copy never autosaves: the planted save probe must
  // stay unconsumed and never surface in the receipt, which names Save
  // instead of a disk write.
  const note = page.getByTestId("textview/copy-note");
  await expect(note).toContainText("Copied hunk 3 take-left");
  await expect(note).toContainText("1 of 1 row(s) merged");
  await expect(note).toContainText("unsaved");
  await expect(note).toContainText("press Save right");
  await expect(note).not.toContainText("AUTOSAVE-PROBE-125");
  await expect(note).not.toContainText("clipboard");
  await expect(note).not.toContainText("saved right to");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  expect(
    (
      (await page.evaluate(
        () => (window as unknown as Record<string, unknown>).__TEXT_SAVE_SIDE_NEXT__,
      )) as { error?: unknown } | null
    )?.error,
  ).toBe("AUTOSAVE-PROBE-125");

  // Hunk resolves visually: row 51 equal, right carries the left text,
  // and the hunk's own gutter pair is gone (ordinals shift, so the
  // remaining pair count drops by one instead of pinning ordinals).
  const leftOf51 = page.getByTestId("textview/row-51").locator('div[title="left"]');
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "equal");
  await expect(rightOf51).toHaveText(await leftOf51.textContent() ?? "");
  await expect(page.getByTestId("textview/row-51").locator('[data-testid^="hunk-copy-"]')).toHaveCount(0);
  await expect.poll(async () => pairCount(), { timeout: 10000 }).toBe(pairsBefore - 1);

  // BC stage-only (wave-172b): nothing is journaled for reload replay.
  expect(await storedCopyRecord(page)).toBeNull();

  // Reload: the staged copy is dropped (BC wins over wave-125
  // persistence) — the hunk is a difference again, no note, no crash.
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await expect(page.getByTestId("textview/row-51")).toHaveAttribute("data-kind", "changed", { timeout: 15000 });
  await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("toolbar Copy runs the same path; toolbar Merge jumps in merge kind", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // No selection yet: Copy parked with the legacy honest reason.
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  expect(await page.getByTestId("toolbar/action-copy").getAttribute("title")).toContain("select a row");

  // Select the changed row 98: Copy enables as take-left.
  await page.getByTestId("textview/row-98").click();
  const copyBtn = page.getByTestId("toolbar/action-copy");
  await expect(copyBtn).toBeEnabled();
  expect(await copyBtn.getAttribute("aria-label")).toContain("take left");

  await copyBtn.click();
  await expect(page.getByTestId("textview/copy-note")).toContainText("Copied hunk 4 take-left");
  await expect(page.getByTestId("textview/copy-note")).toContainText("1 of 1 row(s) merged");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/copy-note")).toContainText("press Save right");
  await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");

  // Equal row selected: Copy parks honestly, same note family.
  await page.getByTestId("textview/row-99").click();
  await expect(copyBtn).toBeDisabled();
  expect(await copyBtn.getAttribute("title")).toContain("not a difference");

  // Merge kind: Merge enables and jumps to the Merge view.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/merge125-left.txt");
  await page.getByTestId("session/right-path").fill("/tmp/merge125-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("view-merge")).toBeVisible();
  const mergeBtn = page.getByTestId("toolbar/action-merge");
  await expect(mergeBtn).toBeEnabled();
  await mergeBtn.click();
  await expect(page.getByTestId("view-merge")).toBeVisible();
  await expect(page.getByTestId("statusbar/message")).toContainText("Merge view");
  // Non-text kind: Copy parks with the legacy reason.
  await expect(copyBtn).toBeDisabled();
  expect(await copyBtn.getAttribute("title")).toContain("select a row");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
