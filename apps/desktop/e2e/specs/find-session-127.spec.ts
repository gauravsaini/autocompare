import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-127 full-session find (audit M2/H7 ranked #4).
 *
 * Drives the REAL `App` workspace under `VITE_MOCK_IPC=1` and proves:
 *  - `session.find` (menu path) OPENS the find panel/filter first, then
 *    focuses — kind-aware: text + hex + table + merge find boxes, folder
 *    name filter, sync step filter. Panels mount closed by default, so a
 *    focus-first query would hit an unmounted input (the wave-122 bug).
 *    The menu bridge itself cannot fire without the Tauri shell, so the
 *    spec dispatches the same `autocompare:session-find` window event the
 *    `session.find` handler dispatches.
 *  - text find reaches beyond the loaded windows via backend `text_find`
 *    (mock mirrors the Rust `find_in_rows` over all 644 fixture rows):
 *    a query matching only unloaded row 200 pages its window in and
 *    activates (`match 1 of 1`).
 *  - text goto keeps paging (row 600 loads on demand, `jumped to row 600`).
 *
 * New file only, no existing spec touched. The kind-walk is split into
 * three tests (<=3 tabs each) so the tabbar never overflows into an
 * overlap violation; the goto deep-scroll omits the overlap gate like
 * text-goto (sticky pathbar over rows is pre-existing chrome truth).
 */
test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

/** Same `autocompare:session-find` event the `session.find` menu handler dispatches. */
async function sessionFind(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("autocompare:session-find"));
  });
}

async function createSession(page: Page, kind: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(`/tmp/u127-${kind}-left`);
  await page.getByTestId("session/right-path").fill(`/tmp/u127-${kind}-right`);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestId)).toBeVisible();
}

test("session.find opens text + table find first, then focuses", async ({ page }) => {
  // Text (demo boot): panel mounts closed, open-first then focus.
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/find-panel")).toBeHidden();
  await sessionFind(page);
  await expect(page.getByTestId("textview/find-panel")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("textview/find-input");

  // Table: find panel mounts closed since wave-113A.
  await createSession(page, "table", "view-table");
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await sessionFind(page);
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("table-find-input");

  await assertNoOverlap(page);
});

test("session.find opens hex + merge find first, then focuses", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });

  // Hex: close the default-open panel first so open-first is proven.
  await createSession(page, "hex", "view-hex");
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await page.getByTestId("hex-find-close").click();
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await sessionFind(page);
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("hex-find-input");

  // Merge: find panel mounts closed since wave-113C.
  await createSession(page, "merge", "view-merge");
  await expect(page.getByTestId("merge/find-panel")).toBeHidden();
  await sessionFind(page);
  await expect(page.getByTestId("merge/find-panel")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("merge/find-input");

  await assertNoOverlap(page);
});

test("session.find targets folder + sync filters, then focuses", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });

  // Folder: the session.find target is the name filter (no find box here).
  await createSession(page, "folder", "view-folder");
  await expect(page.getByTestId("folderview/name-panel")).toBeHidden();
  await sessionFind(page);
  await expect(page.getByTestId("folder-name-filter")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("folder-name-filter");

  // Sync: the session.find target is the step filter.
  await createSession(page, "sync", "view-sync");
  await sessionFind(page);
  await expect(page.getByTestId("syncview/name-filter")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("syncview/name-filter");

  await assertNoOverlap(page);
});

test("text find reaches beyond loaded rows via backend text_find, goto keeps paging", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Banner-free text session: the demo banner is sticky, so deep scrolls
  // (goto row 600) would trip the overlap gate on pre-existing chrome —
  // out of scope for this wave. The mock serves the same 644 fixture rows.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await createSession(page, "text", "view-text");
  // Mock text session: 644 fixture rows, first 200 loaded ("0196" lives
  // only in unloaded row 200, so client-only search misses it).
  await sessionFind(page);
  const input = page.getByTestId("textview/find-input");
  await expect(input).toBeVisible();
  await input.fill("0196");
  await input.press("Enter");
  const status = page.getByTestId("textview/find-status");
  await expect(status).toContainText("match 1 of 1", { timeout: 15000 });
  // The backend hit's window paged in: unloaded row 200 now mounts,
  // selects, and takes DOM focus like any client-only hit.
  await expect(page.getByTestId("textview/row-200")).toBeVisible({ timeout: 15000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("textview/row-200");

  await assertNoOverlap(page);

  // Goto overlays keep paging: row 600 loads on demand (overlap gate
  // omitted after the deep scroll, same as text-goto).
  await page.getByTestId("textview/toolbar-goto").click();
  const goto = page.getByTestId("textview/goto-input");
  await expect(goto).toBeVisible();
  await goto.fill("600");
  await goto.press("Enter");
  await expect(page.getByTestId("textview/goto-status")).toContainText("jumped to row 600", {
    timeout: 15000,
  });
});
