/**
 * Wave-32 table column visibility proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`; header is
 * `id | name | price`, diffs live in the `price` column only) and proves:
 *  - the Columns menu (`table-cols-toggle` -> `table-cols-menu`) lists the
 *    already-loaded header with one checked checkbox per column
 *    (`table-col-visibility-0/1/2`), keyboard reachable (Enter opens,
 *    Space toggles, Escape closes), with `table-cols-show-all` disabled
 *    and counts reading `100 / 550 rows · 3 cols` up front;
 *  - hiding `price` removes the column from both per-side grids (`thead th` 8 -> 6,
 *    `tableview/cell-0-2` gone, `.tc-diff` 10 -> 0) while data stays loaded
 *    (`100 / 550 rows` intact) and counts read honestly
 *    (`2 of 3 cols (1 hidden)`, `1 hidden` status, show-all enabled);
 *  - find skips hidden columns: `10.` (price-only values) reports
 *    `No matches` with zero hit cells plus the inline
 *    `table-find-hidden-note`, where the same query lists hits with all
 *    columns shown;
 *  - show-all restores the grids (`thead th` back to 8, diffs back to 10,
 *    counts back to `3 cols`, find lists `Hit 1 of 6` again, note gone);
 *  - switching sessions clears hidden state (hide a column, create a
 *    second table session, the fresh view shows all 3 columns with
 *    show-all disabled — Remount via `key={activeTabId}` plus the
 *    effect reset, no cross-session leak);
 *  - light + dark PNGs land in `/tmp/ui-32/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock-IPC honesty: visibility is pure component state — no new IPC
 * commands are issued (hiding never refetches). Every assertion reads
 * the DOM, never the network. Skip policy matches the other specs
 * (infra failures skip). New file only; does not touch any existing spec.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-32";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createTableSession(page: Page, left: string, right: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("table columns: hide narrows grid, find skips hidden with note, show-all restores, session switch clears, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Deterministic table tab (clears the Wave-10 demo session first).
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await createTableSession(page, "/tmp/wave32-left.csv", "/tmp/wave32-right.csv");
  const view = page.getByTestId("view-table");

  // Initial: menu closed, all columns shown, honest counts.
  const toggle = page.getByTestId("table-cols-toggle");
  const showAll = page.getByTestId("table-cols-show-all");
  const counts = page.getByTestId("table-cols-count");
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  await expect(showAll).toBeDisabled();
  await expect(counts).toContainText("100 / 550 rows");
  await expect(counts).toContainText("3 cols");
  await expect(page.getByTestId("table-cols-visible")).toContainText("all columns shown");
  await expect(view.locator("thead th")).toHaveCount(8);
  await expect(view.locator(".tc-diff")).toHaveCount(10);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Keyboard: Enter on the focused toggle opens the menu with one checked
  // checkbox per already-loaded header column.
  await toggle.focus();
  await page.keyboard.press("Enter");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  const menu = page.getByTestId("table-cols-menu");
  await expect(menu).toBeVisible();
  const boxId = page.getByTestId("table-col-visibility-0");
  const boxName = page.getByTestId("table-col-visibility-1");
  const boxPrice = page.getByTestId("table-col-visibility-2");
  await expect(boxId).toBeChecked();
  await expect(boxName).toBeChecked();
  await expect(boxPrice).toBeChecked();
  await expect(menu).toContainText("id");
  await expect(menu).toContainText("name");
  await expect(menu).toContainText("price");

  // Keyboard: Space on the focused price checkbox hides that column.
  await boxPrice.focus();
  await page.keyboard.press("Space");
  await expect(boxPrice).not.toBeChecked();

  // Grid narrows, data stays loaded, counts stay honest, diffs follow.
  await expect(view.locator("thead th")).toHaveCount(6);
  await expect(page.getByTestId("tableview/cell-0-2")).toHaveCount(0);
  await expect(view.locator(".tc-diff")).toHaveCount(0);
  await expect(counts).toContainText("100 / 550 rows");
  await expect(counts).toContainText("2 of 3 cols (1 hidden)");
  await expect(page.getByTestId("table-cols-visible")).toContainText("1 hidden");
  await expect(showAll).toBeEnabled();
  // Untouched columns keep their cells + diff-free state.
  await expect(page.getByTestId("tableview/cell-0-0")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Keyboard: Escape closes the menu.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);

  // Find skips the hidden column with an honest inline note: `10.` lives
  // in `price` only, so with price hidden there are no matches.
  // Wave-113A preamble (zero assertion change): find panel is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();
  await page.getByTestId("table-find-input").fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("No matches", {
    timeout: 10000,
  });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);
  const hiddenNote = page.getByTestId("table-find-hidden-note");
  await expect(hiddenNote).toBeVisible();
  await expect(hiddenNote).toContainText("hidden");

  // Show-all restores the grid; the same query lists hits again.
  // (Menu was closed via Escape, so show-all runs from the toolbar row.)
  await showAll.click();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  await expect(view.locator("thead th")).toHaveCount(8);
  await expect(page.getByTestId("tableview/cell-0-2")).toBeVisible();
  await expect(view.locator(".tc-diff")).toHaveCount(10);
  await expect(counts).toContainText("3 cols");
  await expect(page.getByTestId("table-cols-visible")).toContainText("all columns shown");
  await expect(showAll).toBeDisabled();
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 1 of 6", {
    timeout: 10000,
  });
  await expect(page.getByTestId("table-hit-0")).toBeVisible();
  await expect(page.getByTestId("table-find-hidden-note")).toHaveCount(0);

  // Session switch clears hidden state: hide price again, then create a
  // second table session — the fresh view shows all columns.
  await toggle.click();
  await expect(page.getByTestId("table-cols-menu")).toBeVisible();
  await page.getByTestId("table-col-visibility-2").uncheck();
  await expect(counts).toContainText("2 of 3 cols (1 hidden)");
  await createTableSession(page, "/tmp/wave32-b-left.csv", "/tmp/wave32-b-right.csv");
  await expect(page.getByTestId("view-table")).toContainText("100 / 550 rows");
  await expect(page.getByTestId("table-cols-count")).toContainText("3 cols");
  await expect(page.getByTestId("table-cols-show-all")).toBeDisabled();
  await expect(page.getByTestId("table-cols-visible")).toContainText("all columns shown");
  await expect(page.getByTestId("view-table").locator("thead th")).toHaveCount(8);
  await expect(page.getByTestId("table-find-hidden-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo): hide `name` on the fresh
  // session so the shot captures a narrowed grid + find note in both.
  await page.getByTestId("table-cols-toggle").click();
  await page.getByTestId("table-col-visibility-1").uncheck();
  // Wave-113A preamble 2: fresh session after session-switch resets the
  // default-closed find panel; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();
  await page.getByTestId("table-find-input").fill("item-0001");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("No matches", {
    timeout: 10000,
  });
  await expect(page.getByTestId("table-find-hidden-note")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/table-columns-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-columns-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
