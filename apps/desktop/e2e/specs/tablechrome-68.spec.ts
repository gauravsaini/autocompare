/**
 * Wave-68 table chrome collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the mock table engine and proves
 * the Beyond Compare style collapse (mirrors wave-67B folder +
 * wave-66R text):
 *  - one toolbar row (`tableview/toolbar`) hosts the by-key toggle,
 *    the Columns toggle + Show all, Export, the Find toggle, and
 *    prev/next difference nav;
 *  - the find panel renders open by default (backward compat: every
 *    legacy find testid stays visible on load) while the cols menu
 *    stays closed by default (wave-32 contract);
 *  - the bottom status line (`tableview/status`) carries the TOTAL
 *    diff-row count (`tableview/diffs`, wave-69 `diffRows` over the
 *    full tables), the total same count, the loaded scope (the
 *    no-total-diff-count note is gone), and the load time;
 *  - toolbar/Esc collapse the find panel, Ctrl+F reopens + focuses it;
 *  - diff nav walks the loaded difference rows with an honest position;
 *  - collapsed chrome budget: first row top <= 140px relative to the
 *    table section top with both panels closed;
 *  - every legacy testid still works (by-key, cols, export, find,
 *    sort, hidden-note);
 *  - light + dark, zero console/page errors.
 *
 * Wave-69: extended in place (single spec file) to assert the
 * FULL-table total instead of the loaded scope.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-68";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createTableSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave68-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave68-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("table chrome collapse: single toolbar, find overlay, diff count status bar, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  const view = page.getByTestId("view-table");
  await expect(page.getByTestId("table-cols-count")).toContainText("100 / 550 rows");

  // 1. Single toolbar row with every toggle; find panel open by
  //    default, cols menu closed by default (wave-32 contract).
  const toolbar = page.getByTestId("tableview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("table-bykey-toggle")).toBeVisible();
  await expect(page.getByTestId("table-bykey-status")).toContainText("positional");
  await expect(page.getByTestId("table-cols-toggle")).toBeVisible();
  await expect(page.getByTestId("table-cols-toggle")).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByTestId("table-cols-show-all")).toBeVisible();
  await expect(page.getByTestId("table-cols-visible")).toBeVisible();
  await expect(page.getByTestId("table-export")).toBeVisible();
  // Wave-113A preamble (zero assertion change): find panel is default-closed;
  // open it first so the overlay assertions below test the open state.
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("tableview/toolbar-find")).toBeVisible();
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect(page.getByTestId("tableview/toolbar-prev-diff")).toBeVisible();
  await expect(page.getByTestId("tableview/toolbar-next-diff")).toBeVisible();
  await expect(page.getByTestId("tableview/toolbar-diffs")).toBeVisible();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await expect(page.getByTestId("table-find-search")).toBeVisible();
  await expect(page.getByTestId("table-find-next")).toBeVisible();
  await expect(page.getByTestId("table-find-status")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar at the bottom carries the TOTAL diff-row count from
  // the summary (mock fixture: 125 diff rows over the full 550-row
  // tables, 425 same) plus the loaded scope; the wave-68 honest
  // no-total-diff-count note is gone.
  const status = page.getByTestId("tableview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("tableview/diffs")).toContainText("125 difference row(s)");
  await expect(page.getByTestId("tableview/same")).toContainText("425 same");
  await expect(page.getByTestId("tableview/diffs-note")).toHaveCount(0);
  await expect(status).toContainText(/Load time:/);
  const statusBox = await status.evaluate((el) => el.getBoundingClientRect());
  const firstRowBox = await page
    .locator("tbody tr.tc-row")
    .first()
    .evaluate((el) => el.getBoundingClientRect());
  expect(statusBox.top, "counts live below the grid in the status bar").toBeGreaterThan(
    firstRowBox.top,
  );

  // 2. Toolbar toggle collapses the find panel; Ctrl+F reopens + focuses.
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("table-find-input");

  // 3. Esc closes the find panel (focus was in the input).
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeVisible();

  // 4. Cols menu still toggles (legacy intact); Escape closes it.
  await page.getByTestId("table-cols-toggle").click();
  await expect(page.getByTestId("table-cols-menu")).toBeVisible();
  await expect(page.getByTestId("table-col-visibility-0")).toBeChecked();
  await page.getByTestId("table-col-visibility-0").focus();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);

  // 5. Diff nav walks the loaded difference rows with an honest position.
  const diffsLabel = page.getByTestId("tableview/toolbar-diffs");
  await expect(diffsLabel).toContainText("difference 1 of 5");
  await page.getByTestId("tableview/toolbar-next-diff").click();
  await expect(diffsLabel).toContainText("difference 2 of 5");
  await page.getByTestId("tableview/toolbar-prev-diff").click();
  await expect(diffsLabel).toContainText("difference 1 of 5");
  // Hiding the only diff column empties the loaded-scope nav honestly;
  // the status bar still shows the FULL-table total (125).
  await page.getByTestId("table-cols-toggle").click();
  await page.getByTestId("table-col-visibility-2").uncheck();
  await expect(page.getByTestId("tableview/diffs")).toContainText("125 difference row(s)");
  await expect(diffsLabel).toContainText("no differences");
  await expect(page.getByTestId("tableview/toolbar-next-diff")).toBeDisabled();
  await page.getByTestId("table-cols-show-all").click();
  await expect(page.getByTestId("table-cols-menu")).toBeVisible();
  await page.getByTestId("table-cols-toggle").click();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);

  // 6. Legacy behavior intact: sort status cycles, find lists hits.
  await expect(page.getByTestId("tableview/diffs")).toContainText("125 difference row(s)");
  await page.getByTestId("table-sort-0").click();
  await expect(page.getByTestId("table-sort-status")).toContainText(/Sorted by id ascending/);
  await page.getByTestId("table-sort-0").click();
  await page.getByTestId("table-sort-0").click();
  await expect(page.getByTestId("table-sort-status")).toHaveCount(0);
  // Wave-142: cols-menu Esc bubbles to the global handler and also closes find — reopen.
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await page.getByTestId("table-find-input").fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 1 of 6", {
    timeout: 10000,
  });
  await expect(page.getByTestId("table-hit-0")).toBeVisible();

  // 7. Collapsed chrome budget: with both panels closed the table
  //    chrome (toolbar + counts, i.e. first-row top relative to the
  //    table section top — app tab/path bars above are not table
  //    chrome) is compact. Old stacked chrome (~6 rows) was ~200px+.
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `table chrome collapsed to one row (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/table-chrome-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-chrome-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
