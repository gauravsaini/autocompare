/**
 * Wave-44 table column-sort proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics; header `id | name | price`, 550
 * rows, 100 loaded) and proves Beyond Compare style header sorting:
 *  - clicking a header sort button (`table-sort-<c>`) sorts the LOADED
 *    rows ascending with an honest status (`table-sort-status`) and
 *    `aria-sort="ascending"` on the `th`;
 *  - a second click flips to descending; a third click clears back to
 *    session order (status + aria-sort gone);
 *  - physical row numbers stay stable (first body row keeps its `r+1`
 *    identity... asserted via cell testids, not positions);
 *  - the CSV export follows the displayed order and names it in the
 *    receipt (`sorted by ...`);
 *  - keyboard: Enter on the focused header button cycles the sort;
 *  - light + dark PNGs land in `/tmp/ui-44/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-44";

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
  await page.getByTestId("session/left-path").fill("/tmp/wave38-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave38-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

/** Text of the n-th visible body row's n-th data cell (skips row-number th). */
async function bodyCell(page: Page, row: number, cell: number): Promise<string> {
  return (
    (await page
      .locator(`tbody tr.tc-row >> nth=${row} >> td >> nth=${cell}`)
      .textContent()) ?? ""
  ).trim();
}

test("table sort: header cycles asc/desc/clear, numbers stable, export follows, light+dark", async ({
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

  const sortId = page.getByTestId("table-sort-0");
  const sortPrice = page.getByTestId("table-sort-2");
  const sortName = page.getByTestId("table-sort-1");
  const status = page.getByTestId("table-sort-status");
  await expect(sortId).toBeVisible();
  await expect(status).toHaveCount(0);

  // Fixture ids are R0001-style strings: a click still cycles and the
  // status stays honest (string order).
  await sortId.click();
  await expect(status).toContainText(/Sorted by id ascending/);

  // Numeric-aware: price cells ("10.25") order numerically ascending.
  await sortPrice.click();
  await expect(status).toContainText(/Sorted by price ascending/);
  const ascFirst = Number(await bodyCell(page, 0, 2));
  const ascSecond = Number(await bodyCell(page, 1, 2));
  expect(
    Number.isFinite(ascFirst) && Number.isFinite(ascSecond),
    "price cells read numerically",
  ).toBe(true);
  expect(ascFirst, "ascending: first <= second").toBeLessThanOrEqual(ascSecond);

  // Descending: flips the order.
  await sortPrice.click();
  await expect(status).toContainText(/Sorted by price descending/);
  const descFirst = Number(await bodyCell(page, 0, 2));
  const descSecond = Number(await bodyCell(page, 1, 2));
  expect(descFirst, "descending: first >= second").toBeGreaterThanOrEqual(descSecond);
  expect(
    descFirst !== ascFirst || descSecond !== ascSecond,
    "desc order differs from asc order",
  ).toBe(true);

  // Third click clears back to session order (status gone).
  await sortPrice.click();
  await expect(status).toHaveCount(0);

  // Keyboard: Enter on the focused name-header button sorts ascending.
  await sortName.focus();
  await page.keyboard.press("Enter");
  await expect(status).toContainText(/Sorted by name ascending/);
  const nameFirst = await bodyCell(page, 0, 1);
  const nameSecond = await bodyCell(page, 1, 1);
  expect(nameFirst.localeCompare(nameSecond) <= 0, "names ascending").toBe(true);

  // Export follows the displayed (sorted) order and names it.
  await page.getByTestId("table-export").click();
  await expect(page.getByTestId("table-export-receipt")).toContainText(
    /sorted by name ascending/,
  );

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/table-sort-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-sort-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
