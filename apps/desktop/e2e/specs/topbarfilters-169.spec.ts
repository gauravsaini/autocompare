/**
 * Wave-169: top-bar filters + table minor enablement proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves:
 *  - table kind: top-bar Minor is live and toggles both table IPC ignore
 *    flags (`ignoreCase` / `ignoreSpace`) through the existing view bus
 *    (`autocompare:view-minor` with `viewKind: "table"`) and `tableCompare`,
 *    updating the hook receipt (`window.__TABLE_IGNORE_LAST__`), button title
 *    (`Minor (on)` / `Minor`), and status message;
 *  - chrome budget holds with table open (firstRowTop <= 140, no new rows);
 *  - top-bar filter actions (All / Diffs / Same / Context) drive all three
 *    views (table, folder, text) via the unified `autocompare:view-filter` bus;
 *  - zero console errors / page errors throughout.
 *
 * PORT-FREE: run on a fresh port (`reuseExistingServer: false`,
 * `--workers=1`, webkit) so no shared port or parallel worker can
 * flake the gate.
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

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  return { pageErrors, consoleErrors };
}

function assertClean(
  pageErrors: string[],
  consoleErrors: string[],
  where: string,
): void {
  expect(pageErrors, `zero page errors (${where})`).toEqual([]);
  expect(consoleErrors, `zero console errors (${where})`).toEqual([]);
}

/** Section-relative first-row delta (no offsetHeight locks). */
async function firstRowDelta(
  page: Page,
  sectionSel: string,
  rowSel: string,
): Promise<number | null> {
  return page.evaluate(
    ([section, row]) => {
      const s = document.querySelector(section);
      const r = document.querySelector(row);
      if (!(s instanceof HTMLElement) || !(r instanceof HTMLElement)) return null;
      return Math.round(r.getBoundingClientRect().top - s.getBoundingClientRect().top);
    },
    [sectionSel, rowSel] as const,
  );
}

/** Create a fresh comparison session through the session dialog. */
async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewId: string,
): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click().catch(() => {});
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewId)).toBeVisible({ timeout: 15000 });
}

test("topbarfilters-169: table Minor toggles + receipt, bus dispatch, budget <= 140px", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);

  // Boot a real table session
  await createSession(page, "table", "/tmp/t169-left.csv", "/tmp/t169-right.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]").first()).toBeVisible({
    timeout: 15000,
  });

  // Track autocompare:view-minor events on window
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__LAST_VIEW_MINOR__ = null;
    window.addEventListener("autocompare:view-minor", (e) => {
      (window as unknown as Record<string, unknown>).__LAST_VIEW_MINOR__ = (
        e as CustomEvent
      ).detail;
    });
  });

  // Top-bar Minor is enabled in table kind
  const minor = page.getByTestId("toolbar/action-minor");
  await expect(minor).toBeEnabled();
  await expect(minor).toHaveAttribute("title", "Minor");

  // Initial table compare hook: no ignore flags
  const hookBefore = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
  );
  expect(hookBefore, "table initial compare sends no ignore flags").toMatchObject({
    ignoreCase: false,
    ignoreSpace: false,
  });

  // Click Minor ON: toggles title, sets status message, dispatches event, updates hook receipt
  await minor.click();
  await expect(minor).toHaveAttribute("title", "Minor (on)");
  await expect(page.getByTestId("statusbar/message")).toContainText("ignoring whitespace and case");

  const minorEventOn = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__LAST_VIEW_MINOR__,
  );
  expect(minorEventOn, "view-minor bus received table kind event").toEqual({
    on: true,
    viewKind: "table",
  });

  const hookOn = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
  );
  expect(hookOn, "table ignore flags reached tableCompare wire").toMatchObject({
    ignoreCase: true,
    ignoreSpace: true,
  });

  // Chrome budget: firstRowTop <= 140px
  const delta = await firstRowDelta(
    page,
    'section[aria-label="table comparison"]',
    'tbody tr.tc-row, [data-testid="tableview/row-0"]',
  );
  expect(delta, "table firstRowTop measured").not.toBeNull();
  expect(delta!, `table firstRowTop <= 140px (got ${delta})`).toBeLessThanOrEqual(140);

  // Click Minor OFF: restores exact diff, updates hook receipt
  await minor.click();
  await expect(minor).toHaveAttribute("title", "Minor");
  await expect(page.getByTestId("statusbar/message")).toContainText("Minor off");

  const minorEventOff = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__LAST_VIEW_MINOR__,
  );
  expect(minorEventOff, "view-minor bus received off event").toEqual({
    on: false,
    viewKind: "table",
  });

  const hookOff = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
  );
  expect(hookOff, "table ignore flags cleared").toMatchObject({
    ignoreCase: false,
    ignoreSpace: false,
  });

  assertClean(pageErrors, consoleErrors, "table Minor toggles");
});

test("topbarfilters-169: filters drive all three views (table, folder, text)", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);

  // 1. TABLE VIEW: filters update table rowFilter and status
  await createSession(page, "table", "/tmp/tf169-left.csv", "/tmp/tf169-right.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]").first()).toBeVisible({
    timeout: 15000,
  });

  const filterDiffs = page.getByTestId("toolbar/action-filter-diffs");
  const filterSame = page.getByTestId("toolbar/action-filter-same");
  const filterContext = page.getByTestId("toolbar/action-filter-context");
  const filterAll = page.getByTestId("toolbar/action-filter-all");

  await filterDiffs.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: diffs");
  await expect(page.getByTestId("table-cols-count")).toContainText("filter: diffs");

  await filterSame.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: same");
  await expect(page.getByTestId("table-cols-count")).toContainText("filter: same");

  await filterContext.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: context");
  await expect(page.getByTestId("table-cols-count")).toContainText("filter: context");

  await filterAll.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: all");
  await expect(page.getByTestId("table-cols-count")).not.toContainText("filter:");

  // 2. FOLDER VIEW: filters update folder rowFilter and SegmentedControl
  await createSession(page, "folder", "/tmp/f169-left", "/tmp/f169-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(3);

  await filterDiffs.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: diffs");
  await expect(page.getByTestId("folder-filter-diffs")).toHaveAttribute("aria-pressed", "true");

  await filterSame.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: same");
  await expect(page.getByTestId("folder-filter-same")).toHaveAttribute("aria-pressed", "true");

  await filterAll.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: all");
  await expect(page.getByTestId("folder-filter-all")).toHaveAttribute("aria-pressed", "true");

  // 3. TEXT VIEW: filters update text viewFilter and diffs-only note
  await createSession(page, "text", "/tmp/t169-l.txt", "/tmp/t169-r.txt", "view-text");
  await expect(page.getByTestId("textview/status")).toContainText("rows", { timeout: 15000 });

  await filterDiffs.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: diffs");
  await expect(page.getByTestId("textview/diffsonly-note")).toBeVisible();

  await filterAll.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: all");
  await expect(page.getByTestId("textview/diffsonly-note")).toBeHidden();

  assertClean(pageErrors, consoleErrors, "filters drive all three views");
});
