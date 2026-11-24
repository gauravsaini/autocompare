/**
 * Wave-66M table key-column picker proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`, header is
 * `id | name | price`) and proves:
 *  - every header cell renders a small `key` toggle (`table-key-<c>`,
 *    `aria-pressed="false"`) and the view boots positional with no
 *    explicit request (`window.__TABLE_KEYCOLS_LAST__ === []`);
 *  - clicking `table-key-1` requests that column explicitly
 *    (`__TABLE_KEYCOLS_LAST__ === [1]`, `aria-pressed="true"`),
 *    enables key mode (by-key toggle checked, status `by-key`), the
 *    grid reloads under the new session (`100 / 550 rows`, `.tc-diff`
 *    cells present), and — because the mock fixture echoes
 *    `byKey: false, keyColumns: []` — the honest wave-24 fallback note
 *    (`table-bykey-note`) appears with zero key-column highlights;
 *  - clicking `table-key-1` again clears to auto (`[]`, `aria-pressed`
 *    back to `false`, grid intact); unchecking the by-key toggle drops
 *    the designation and restores positional mode;
 *  - light + dark PNGs land in `/tmp/ui-66m/` (never in-repo);
 *  - `assertNoOverlap` holds and zero console/page errors occur.
 *
 * Mock-IPC honesty: the mock adapter has no key-alignment engine and
 * `src/ipc/client.ts` is wave-frozen, so the request path is proven the
 * same way `table-bykey.spec.ts` proves its IPC param — via the
 * headless-proof hook the view mirrors (`__TABLE_KEYCOLS_LAST__`,
 * alongside `__TABLE_BYKEY_LAST__`) — plus the graceful positional
 * fallback, never a crash. The REAL explicit-honoring path is covered
 * by the `table_compare` Rust unit tests. Skip policy matches the other
 * specs (infra failures skip). New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-66m";

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
  await page.getByTestId("session/left-path").fill("/tmp/wave66m-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave66m-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function lastKeyCols(page: Page): Promise<unknown> {
  return page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_KEYCOLS_LAST__,
  );
}

async function lastByKey(page: Page): Promise<unknown> {
  return page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_BYKEY_LAST__,
  );
}

test("table key-column picker: toggle requests column, clearing returns to auto, light+dark", async ({
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

  // Boot: positional, one key toggle per header column, nothing requested.
  const key0 = page.getByTestId("table-key-0");
  const key1 = page.getByTestId("table-key-1");
  const key2 = page.getByTestId("table-key-2");
  await expect(key0).toBeVisible();
  await expect(key1).toBeVisible();
  await expect(key2).toBeVisible();
  await expect(key0).toHaveAttribute("aria-pressed", "false");
  await expect(key1).toHaveAttribute("aria-pressed", "false");
  await expect(key2).toHaveAttribute("aria-pressed", "false");
  await expect.poll(() => lastKeyCols(page)).toEqual([]);
  await expect.poll(() => lastByKey(page)).toBe(false);
  await expect(page.getByTestId("table-bykey-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Toggle ON: explicit key_columns:[1] request path, key mode enabled,
  // grid reloads, honest fallback note (mock echoes positional).
  await key1.click();
  await expect(key1).toHaveAttribute("aria-pressed", "true");
  await expect(key0).toHaveAttribute("aria-pressed", "false");
  await expect(key2).toHaveAttribute("aria-pressed", "false");
  await expect.poll(() => lastKeyCols(page)).toEqual([1]);
  await expect.poll(() => lastByKey(page)).toBe(true);
  await expect(page.getByTestId("table-bykey-toggle")).toBeChecked();
  await expect(page.getByTestId("table-bykey-status")).toContainText("by-key");
  await expect(view).toContainText("100 / 550 rows");
  await expect(page.getByTestId("table-bykey-note")).toBeVisible();
  await expect(page.getByTestId("table-bykey-note")).toContainText("positional");
  await expect(view.locator('[data-key="true"]')).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
  const diffCountKeyed = await view.locator(".tc-diff").count();
  expect(diffCountKeyed).toBeGreaterThan(0);

  // Clearing returns to auto: same button, request path back to [].
  await key1.click();
  await expect(key1).toHaveAttribute("aria-pressed", "false");
  await expect.poll(() => lastKeyCols(page)).toEqual([]);
  await expect(view).toContainText("100 / 550 rows");
  await expect(page.getByRole("alert")).toHaveCount(0);
  const diffCountCleared = await view.locator(".tc-diff").count();
  expect(diffCountCleared).toBeGreaterThan(0);

  // Unchecking the by-key toggle drops the mode (and any designation).
  await page.getByTestId("table-bykey-toggle").uncheck();
  await expect(page.getByTestId("table-bykey-status")).toContainText("positional");
  await expect.poll(() => lastByKey(page)).toBe(false);
  await expect.poll(() => lastKeyCols(page)).toEqual([]);
  await expect(page.getByTestId("table-bykey-note")).toHaveCount(0);
  await expect(view).toContainText("100 / 550 rows");

  // Light + dark evidence (never in-repo). Re-enable the explicit key
  // so the pressed toggle + fallback note are captured in both themes.
  await key1.click();
  await expect(key1).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("table-bykey-note")).toBeVisible();
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/tablekeypick-66m-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/tablekeypick-66m-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
