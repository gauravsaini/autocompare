/**
 * Wave-66G table numeric-aware find proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`, header is
 * `id | name | price`) and proves:
 *  - a numeric query hits a numerically-equal-but-string-different cell:
 *    `10.250` is not a substring of `10.25` yet parses to the same f64,
 *    so status reads `Hit 1 of 2` (left + right share one rendered cell)
 *    and the current hit contains `10.25`;
 *  - plain substring still works: `item-0001` lists `Hit 1 of 2`;
 *  - a non-numeric miss names the scope honestly: `10,25` (comma
 *    NaN-parses, substring only) reports `No matches` with zero hits;
 *  - hidden-column skip still holds: hiding `price` makes `10.250`
 *    report `No matches` plus the inline `table-find-hidden-note`, and
 *    show-all restores `Hit 1 of 2` with the note gone;
 *  - an empty query is a clean no-op (hits cleared, no alert, no crash);
 *  - light + dark PNGs land in `/tmp/ui-66g/` (never in-repo);
 *  - `assertNoOverlap` holds and zero console/page errors occur.
 *
 * Mock-IPC honesty: the `table_find` fixture implements the identical
 * numeric-aware predicate over the loaded rows (left then right,
 * row-major, cap 100); IPC knows nothing about visibility (the view
 * filters hidden columns client-side). Every assertion reads the DOM,
 * never the network. Skip policy matches the other specs (infra
 * failures skip). New file only; does not touch any existing spec.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-66g";

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
  await page.getByTestId("session/left-path").fill("/tmp/wave66g-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave66g-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("table numeric find: numeric hit, substring, honest miss, hidden skip, empty no-op, light+dark", async ({
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
  await expect(view).toContainText("100 / 550 rows");
  // Wave-113A preamble (zero assertion change): find panel is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();

  const input = page.getByTestId("table-find-input");
  const next = page.getByTestId("table-find-next");
  const status = page.getByTestId("table-find-status");
  await expect(input).toBeVisible();
  await expect(next).toBeVisible();

  // Numeric query hits the numerically-equal-but-string-different cell:
  // "10.250" is not a substring of "10.25" but parses to the same f64.
  await input.fill("10.250");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("Hit 1 of 2", { timeout: 10000 });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(1);
  const hit0 = page.getByTestId("table-hit-0");
  await expect(hit0).toBeVisible();
  await expect(hit0).toHaveAttribute("data-hit", "true");
  await expect(hit0).toContainText("10.25");
  await expect(next).toBeEnabled();

  // Plain substring still works.
  await input.fill("item-0001");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("Hit 1 of 2", { timeout: 10000 });
  await expect(page.getByTestId("table-hit-0")).toContainText("item-0001");

  // Non-numeric miss names the scope honestly: "10,25" comma-parses to
  // NaN, so it falls back to substring only and matches nothing.
  await input.fill("10,25");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("No matches", { timeout: 10000 });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);
  await expect(next).toBeDisabled();

  // Hidden-column skip still holds: hiding `price` removes the only
  // column holding "10.25", so the numeric query misses with a note.
  await page.getByTestId("table-cols-toggle").click();
  await expect(page.getByTestId("table-cols-menu")).toBeVisible();
  await page.getByTestId("table-col-visibility-2").uncheck();
  await expect(page.getByTestId("table-cols-count")).toContainText("2 of 3 cols (1 hidden)");
  await input.fill("10.250");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("No matches", { timeout: 10000 });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);
  const hiddenNote = page.getByTestId("table-find-hidden-note");
  await expect(hiddenNote).toBeVisible();
  await expect(hiddenNote).toContainText("hidden");

  // Show-all restores the numeric hit and clears the note.
  await page.getByTestId("table-cols-show-all").click();
  await expect(page.getByTestId("table-cols-count")).toContainText("3 cols");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("Hit 1 of 2", { timeout: 10000 });
  await expect(page.getByTestId("table-hit-0")).toBeVisible();
  await expect(page.getByTestId("table-find-hidden-note")).toHaveCount(0);

  // Empty query is a clean no-op: hits cleared, no alert, no crash.
  await input.fill("");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.locator('[data-hit="true"]')).toHaveCount(0);
  await expect(view).toContainText("100 / 550 rows");

  // Light + dark evidence (never in-repo).
  await input.fill("10.250");
  await page.getByTestId("table-find-search").click();
  await expect(status).toContainText("Hit 1 of 2");
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/tablefindnum-66g-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/tablefindnum-66g-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
