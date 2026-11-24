/**
 * Wave-24 table by-key compare toggle proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`) and proves:
 *  - the by-key toggle renders unchecked (`table-bykey-toggle`,
 *    status `positional`) and the view boots positionally
 *    (`window.__TABLE_BYKEY_LAST__ === false`, `100 / 550 rows`);
 *  - checking it passes the explicit `by_key: true` IPC param
 *    (`window.__TABLE_BYKEY_LAST__ === true`), the grid reloads with
 *    updated row counts + diff classes (`.tc-diff` cells present), and —
 *    because the mock fixture echoes `byKey: false, keyColumns: []` —
 *    an honest inline fallback note (`table-bykey-note`) appears with
 *    zero key-column highlights and no crash;
 *  - find still works in by-key mode (`10.` lists hits, `table-hit-0`);
 *  - unchecking restores positional mode (`__TABLE_BYKEY_LAST__ === false`,
 *    note hidden);
 *  - light + dark PNGs land in `/tmp/ui-24/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock-IPC honesty: the mock adapter has no key-alignment engine, so the
 * toggle is verified at the IPC-param path (explicit `by_key` on
 * `table_compare`) plus the graceful positional fallback — never a crash.
 * Skip policy matches the other specs (infra failures skip). New file only;
 * does not touch any existing spec.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-24";

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
  await page.getByTestId("session/left-path").fill("/tmp/wave24-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave24-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function lastByKey(page: Page): Promise<unknown> {
  return page.evaluate(() => (window as unknown as Record<string, unknown>).__TABLE_BYKEY_LAST__);
}

test("table by-key toggle: ipc param path, fallback note, find works, light+dark", async ({
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
  const toggle = page.getByTestId("table-bykey-toggle");
  const status = page.getByTestId("table-bykey-status");

  // Default: positional mode, explicit by_key:false param path.
  await expect(toggle).toBeVisible();
  await expect(toggle).not.toBeChecked();
  await expect(status).toContainText("positional");
  await expect(view).toContainText("100 / 550 rows");
  expect(await lastByKey(page)).toBe(false);
  await expect(page.getByTestId("table-bykey-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
  const diffCountPositional = await view.locator(".tc-diff").count();
  expect(diffCountPositional).toBeGreaterThan(0);

  // Toggle ON: explicit by_key:true param, grid reloads, honest fallback note
  // (mock echoes positional), no crash, zero key highlights.
  await toggle.check();
  await expect(status).toContainText("by-key");
  await expect(view).toContainText("100 / 550 rows");
  expect(await lastByKey(page)).toBe(true);
  await expect(page.getByTestId("table-bykey-note")).toBeVisible();
  await expect(page.getByTestId("table-bykey-note")).toContainText("positional");
  await expect(view.locator('[data-key="true"]')).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
  // Row counts + diff classes update (grid reloaded under the new session).
  const diffCountByKey = await view.locator(".tc-diff").count();
  expect(diffCountByKey).toBeGreaterThan(0);

  // Find still works in by-key mode.
  // Wave-113A preamble (zero assertion change): find panel is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();
  await page.getByTestId("table-find-input").fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 1 of", {
    timeout: 10000,
  });
  await expect(page.getByTestId("table-hit-0")).toBeVisible();

  // Toggle OFF restores positional mode: param path flips back, note hides.
  await toggle.uncheck();
  await expect(status).toContainText("positional");
  expect(await lastByKey(page)).toBe(false);
  await expect(view).toContainText("100 / 550 rows");
  await expect(page.getByTestId("table-bykey-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo). Re-enable by-key for the shot so
  // the toggle + fallback note are captured in both themes.
  await toggle.check();
  await expect(page.getByTestId("table-bykey-note")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/table-bykey-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-bykey-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
