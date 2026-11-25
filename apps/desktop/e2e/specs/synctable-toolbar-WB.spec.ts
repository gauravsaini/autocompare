/**
 * Wave-WB sync+table toolbar parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Pins the SyncToolbar/TableToolbar migration onto shared ui primitives
 * (BC parity, styling-only — persist/IPC wiring byte-identical):
 *  - sync direction rides `ui/SegmentedControl` (three segments, same
 *    testids): boots left-wins pressed, click flips `aria-pressed` and
 *    re-plans with explicit opts on `window.__SYNC_OPTS_LAST__`; prune
 *    checkbox re-plans with `prune:true`; mode note echoes the combo;
 *  - table options ride ui primitives (`Checkbox`/`Select`/`Input`,
 *    `UnifiedNavCluster` for diff nav): defaults boot from
 *    `ac.viewstate.v1.table`, edits survive reload into a NEW table
 *    session (persist round-trip), corrupt storage falls back to
 *    defaults; sync direction/prune stay unpersisted (defaults boot);
 *  - light + dark PNGs land in `/tmp/synctable-WB/` (never in-repo);
 *  - `assertNoOverlap` holds and zero console/page errors occur.
 *
 * Mock honesty: mock `sync_plan`/`table_compare` ignore options (fixed
 * fixtures), so the request path is proven via the headless-proof
 * hooks (`__SYNC_OPTS_LAST__` / `__TABLE_PICOPTS_LAST__`) plus grid
 * reload, never a crash. Skip policy matches the other specs (infra
 * failures skip). New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/synctable-WB";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function assertClean(consoleErrors: string[], pageErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

async function createSyncSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("sync");
  await page.getByTestId("session/left-path").fill("/tmp/wave17-left-dir");
  await page.getByTestId("session/right-path").fill("/tmp/wave17-right-dir");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-sync")).toBeVisible();
}

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

test("sync segments: direction SegmentedControl flips + prune re-plans, light+dark", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await createSyncSession(page);

  const dirLeft = page.getByTestId("syncview/dir-left");
  const dirRight = page.getByTestId("syncview/dir-right");
  const dirNewer = page.getByTestId("syncview/dir-newer");
  const pruneToggle = page.getByTestId("syncview/prune-toggle");
  const modeNote = page.getByTestId("syncview/mode-note");

  // Boot: left-wins segment pressed, prune off, default wire (no opts).
  await expect(dirLeft).toBeVisible();
  await expect(dirLeft).toHaveAttribute("aria-pressed", "true");
  await expect(dirRight).toHaveAttribute("aria-pressed", "false");
  await expect(dirNewer).toHaveAttribute("aria-pressed", "false");
  await expect(pruneToggle).not.toBeChecked();
  await expect
    .poll(
      () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__SYNC_OPTS_LAST__,
        ),
      { timeout: 10000 },
    )
    .toBeNull();

  // Segment click flips pressed + re-plans with explicit opts.
  await dirNewer.click();
  await expect(dirNewer).toHaveAttribute("aria-pressed", "true");
  await expect(dirLeft).toHaveAttribute("aria-pressed", "false");
  await expect
    .poll(
      () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__SYNC_OPTS_LAST__,
        ),
      { timeout: 10000 },
    )
    .toEqual({ direction: "NewerWins", prune: false });
  await expect(modeNote).toContainText("newer", { ignoreCase: true });

  // Prune checkbox re-plans with prune:true (same SegmentedControl row).
  await pruneToggle.check();
  await expect
    .poll(
      () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__SYNC_OPTS_LAST__,
        ),
      { timeout: 10000 },
    )
    .toEqual({ direction: "NewerWins", prune: true });

  await expect(page.getByRole("alert")).toHaveCount(0);
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/sync-toolbar-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.getByTestId("view-sync")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/sync-toolbar-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });
  assertClean(consoleErrors, pageErrors);
});

test("table options: ui primitives + persist round-trip, light+dark", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await createTableSession(page, "/tmp/wb-t-left.csv", "/tmp/wb-t-right.csv");
  const view = page.getByTestId("view-table");
  await expect(view).toContainText("100 / 550 rows");

  // Options ride ui primitives with persisted defaults on boot.
  const byKey = page.getByTestId("table-bykey-toggle");
  const ignoreWs = page.getByTestId("tableview/ignore-ws");
  const ignoreCase = page.getByTestId("tableview/ignore-case");
  const delimiter = page.getByTestId("tableview/delimiter-select");
  const epsilon = page.getByTestId("tableview/numeric-tolerance");
  await expect(byKey).toBeVisible();
  await expect(ignoreWs).not.toBeChecked();
  await expect(ignoreCase).not.toBeChecked();
  await expect(delimiter).toHaveValue(",");
  await expect(epsilon).toHaveValue("");

  // Diff nav rides the shared cluster (same testids, bus-first).
  await expect(page.getByTestId("tableview/toolbar-prev-diff")).toBeVisible();
  await expect(page.getByTestId("tableview/toolbar-next-diff")).toBeVisible();

  // Options set karo: ignore flags + delimiter + epsilon.
  await ignoreWs.check();
  await expect(ignoreWs).toBeChecked();
  await delimiter.selectOption(";");
  await expect
    .poll(
      async () =>
        (
          (await page.evaluate(
            () => (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"],
          )) as Record<string, unknown>
        ).delimiter,
      { timeout: 10000 },
    )
    .toBe(";");
  await epsilon.fill("0.5");
  await expect
    .poll(
      async () =>
        (
          (await page.evaluate(
            () => (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"],
          )) as Record<string, unknown>
        ).numericTolerance,
      { timeout: 10000 },
    )
    .toBe(0.5);
  await expect(view).toContainText("100 / 550 rows");
  const stored = await page.evaluate(() =>
    window.localStorage.getItem("ac.viewstate.v1.table"),
  );
  expect(stored, "table options persisted to localStorage").toContain(";");

  // Reload -> naya table session -> controls abhi bhi set (round-trip).
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await createTableSession(page, "/tmp/wb-t-left2.csv", "/tmp/wb-t-right2.csv");
  await expect(page.getByTestId("tableview/ignore-ws")).toBeChecked();
  await expect(page.getByTestId("tableview/delimiter-select")).toHaveValue(";");
  await expect(page.getByTestId("tableview/numeric-tolerance")).toHaveValue("0.5");

  // Corrupt storage -> defaults wapas.
  await page.evaluate(() => {
    window.localStorage.setItem("ac.viewstate.v1.table", "{corrupt!!");
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await createTableSession(page, "/tmp/wb-t-left3.csv", "/tmp/wb-t-right3.csv");
  await expect(page.getByTestId("tableview/ignore-ws")).not.toBeChecked();
  await expect(page.getByTestId("tableview/delimiter-select")).toHaveValue(",");
  await expect(page.getByTestId("tableview/numeric-tolerance")).toHaveValue("");

  await expect(page.getByRole("alert")).toHaveCount(0);
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/table-toolbar-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.getByTestId("view-table")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/table-toolbar-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });
  assertClean(consoleErrors, pageErrors);
});
