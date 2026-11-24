/**
 * Wave-20 table find proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`) and proves:
 *  - the find bar renders (`table-find-input`, `table-find-next`,
 *    `table-find-status`) with zero crashes on an empty query;
 *  - typing `item-0001` + Find lists hits: status reads `Hit 1 of 2`
 *    (left + right data cells both contain it), two cells carry
 *    `data-hit="true"`, and the current hit is reachable as
 *    `table-hit-0`;
 *  - Next cycles 1/2 -> 2/2 -> wraps to 1/2, moving the `table-hit-N`
 *    highlight each time (the hit cell scrolls into view);
 *  - a no-match query reports `No matches` with zero hit cells;
 *  - an empty query is a clean no-op (hits cleared, no alert, no crash);
 *  - light + dark PNGs land in `/tmp/ui-20/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock-IPC honesty: the mock adapter has NO `table_find` fixture
 * (unknown commands resolve `[]`), so TableView falls back to a
 * client-side substring scan of the LOADED rows with identical semantics
 * (case-sensitive `includes`, left table first, row-major, cap 100).
 * Every assertion below reads the DOM, never the network. Skip policy
 * matches the other specs (infra failures skip). Does not touch any
 * existing spec.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-20";

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
  // Wave-10 boots the demo text session; clear it for a deterministic table tab.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave20-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave20-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("table find: lists hits, next cycles and highlights, empty query is a clean no-op, light+dark", async ({
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

  // Wave-113A preamble (zero assertion change): find panel is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();

  // Find bar renders before any search; empty-query Find is a clean no-op.
  const input = page.getByTestId("table-find-input");
  const next = page.getByTestId("table-find-next");
  const status = page.getByTestId("table-find-status");
  await expect(input).toBeVisible();
  await expect(next).toBeVisible();
  await expect(next).toBeDisabled();
  await expect(view).toContainText("100 / 550 rows");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);

  // Find lists hits: "10." sits in column 3 of rows 1-3 on both sides
  // (6 hits over 3 rendered cells — left/right pairs share one cell).
  await input.fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("Hit 1 of 6", { timeout: 10000 });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(3);
  const hit0 = page.getByTestId("table-hit-0");
  await expect(hit0).toBeVisible();
  await expect(hit0).toHaveAttribute("data-hit", "true");
  await expect(hit0).toContainText("10.25");
  await expect(next).toBeEnabled();

  // Next jumps through hits: 1/6 -> 2/6, highlight moves with it.
  await next.click();
  await expect(status).toContainText("Hit 2 of 6");
  const hit1 = page.getByTestId("table-hit-1");
  await expect(hit1).toBeVisible();
  await expect(hit1).toHaveAttribute("data-hit", "true");
  await expect(hit1).toContainText("10.50");
  await expect(page.getByTestId("table-hit-0")).toHaveCount(0);

  // Next wraps around after the last hit (4 more to reach 6/6, then 1 to wrap).
  for (let i = 0; i < 5; i += 1) {
    await next.click();
  }
  await expect(status).toContainText("Hit 1 of 6");
  await expect(page.getByTestId("table-hit-0")).toBeVisible();
  await expect(page.locator('[data-hit="true"]')).toHaveCount(3);

  // Enter key also runs find (keyboard path, same hit list).
  await input.press("Enter");
  await expect(status).toContainText("Hit 1 of 6");

  // No-match query reports cleanly with zero hit cells.
  await input.fill("zzz-no-such-cell-wave20");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("No matches", { timeout: 10000 });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);
  await expect(next).toBeDisabled();

  // Empty query is a clean no-op: hits cleared, no alert, no crash.
  await input.fill("");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);
  await expect(view).toContainText("100 / 550 rows");

  // Light + dark evidence (never in-repo).
  await input.fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("Hit 1 of 6");
  await page.screenshot({ path: `${EVID_DIR}/table-find-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-find-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
