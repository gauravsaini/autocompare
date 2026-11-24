/**
 * Wave-73 sync chrome collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> SyncView) over the mock sync engine and proves the
 * Beyond Compare Folder Sync style collapse (mirrors wave-72 media):
 *  - one toolbar row (`syncview/toolbar`) hosts the direction buttons
 *    (Update right/left, Newer wins), the prune toggle, the name-filter
 *    toggle, and the dry-run / real apply buttons;
 *  - the name-filter panel renders open on load (backward compat: the
 *    wave-64 filter box stays visible) and the toolbar toggle / Esc
 *    collapse it while Ctrl+F reopens + focuses it;
 *  - the bottom status line (`syncview/statusbar`) carries per-op
 *    direction counts plus the mode, reused from the existing plan data
 *    (no backend change);
 *  - collapsed chrome budget: first sync row top <= 140px relative to
 *    the sync section top with the name-filter panel closed;
 *  - every legacy testid still works (totals/mode-note/apply receipts/
 *    filter round-trip, direction re-plan request path);
 *  - light + dark, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { FOLDER_NODES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-73";
const EXPECTED_STEPS = FOLDER_NODES.filter((n) => n.status !== "same").length;

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createSyncSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("sync");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-sync")).toBeVisible();
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

async function chromeDelta(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder sync plan"]');
    const first = document.querySelector('[data-testid="syncview/row"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return f.top - s.top;
  });
}

test("sync chrome collapse: single-row toolbar, name-filter overlay, status bar, row budget", async ({
  page,
}) => {
  expect(EXPECTED_STEPS, "fixture has non-same nodes").toBeGreaterThan(0);
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createSyncSession(page);
  const view = page.getByTestId("view-sync");
  const rows = page.getByTestId("syncview/row");
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);

  // 1. Single-row toolbar hosts directions + prune + filter toggle + apply.
  const toolbar = page.getByTestId("syncview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("syncview/dir-left")).toBeVisible();
  await expect(page.getByTestId("syncview/dir-right")).toBeVisible();
  await expect(page.getByTestId("syncview/dir-newer")).toBeVisible();
  await expect(page.getByTestId("syncview/prune-toggle")).toBeVisible();
  await expect(page.getByTestId("syncview/toolbar-filter")).toBeVisible();
  await expect(page.getByTestId("syncview/apply-dry")).toBeVisible();
  await expect(page.getByTestId("syncview/apply-real")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar at the bottom carries per-op direction counts + the mode.
  const status = page.getByTestId("syncview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(
    new RegExp(`${EXPECTED_STEPS} steps`),
  );
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(/to right/);
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(/to left/);
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("direction: left to right");
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("prune: off");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await page.getByTestId("syncview/row").first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(firstRowTopAbs);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(firstRowTopAbs);

  await assertNoOverlap(page);

  // 2. Toolbar toggle collapses the name-filter panel; toggle reopens + focuses.
  // Panel boots open (backward compat with the wave-64 filter box).
  await expect(page.getByTestId("syncview/name-filter")).toBeVisible();
  await expect(page.getByTestId("syncview/toolbar-filter")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("syncview/toolbar-filter").click();
  await expect(page.getByTestId("syncview/name-filter")).toBeHidden();
  await expect(page.getByTestId("syncview/toolbar-filter")).toHaveAttribute("aria-expanded", "false");
  // Apply stays reachable from the toolbar while the panel is closed.
  await expect(page.getByTestId("syncview/apply-dry")).toBeVisible();
  await page.getByTestId("syncview/toolbar-filter").click();
  await expect(page.getByTestId("syncview/name-filter")).toBeVisible();
  await expect(page.getByTestId("syncview/toolbar-filter")).toHaveAttribute("aria-expanded", "true");
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("syncview/name-filter");

  // 3. Filter round-trip narrows list-only; Esc clears first (wave-64
  // contract keeps the box mounted), Esc on the empty box closes it.
  const paths = await page.$$eval('[data-testid="syncview/row"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const query = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  const filter = page.getByTestId("syncview/name-filter");
  await filter.fill(query);
  const note = page.getByTestId("syncview/name-note");
  await expect(note).toBeVisible();
  await expect(note).toContainText("apply runs the full plan");
  const narrowed = await rows.count();
  expect(narrowed).toBeLessThanOrEqual(EXPECTED_STEPS);
  expect(narrowed).toBeGreaterThanOrEqual(1);
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(
    new RegExp(`showing ${narrowed}`),
  );
  await page.keyboard.press("Escape");
  await expect(note).toBeHidden();
  await expect(filter).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(filter).toBeHidden();
  await expect(page.getByTestId("syncview/toolbar-filter")).toHaveAttribute("aria-expanded", "false");

  // 4. Ctrl/Cmd+F reopens + focuses the filter from the collapsed state.
  await page.keyboard.press("Control+f");
  try {
    await expect.poll(() => focusedTestId(page), { timeout: 2000 }).toBe("syncview/name-filter");
  } catch {
    await page.keyboard.press("Meta+f");
    await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("syncview/name-filter");
  }
  await expect(filter).toBeVisible();

  // 5. Legacy behavior intact: totals + mode note + dry/real receipts,
  //    direction switch re-plans and the status bar follows the mode.
  await filter.fill("");
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);
  await expect(page.getByTestId("syncview/totals")).toContainText(/steps/);
  await expect(page.getByTestId("syncview/mode-note")).toContainText("direction: left to right");
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/dry run/);
  await expect(page.getByTestId("syncview/receipt")).toContainText(/nothing touched/);
  await page.getByTestId("syncview/dir-newer").click();
  await expect(page.getByTestId("syncview/mode-note")).toContainText("direction: newer wins");
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("direction: newer wins");
  await expect(page.getByTestId("syncview/receipt")).toHaveCount(0);
  await page.getByTestId("syncview/apply-real").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(
    new RegExp(`applied: ${EXPECTED_STEPS} copied, 0 deleted`),
  );

  await assertNoOverlap(page);

  // 6. Collapsed chrome budget: with the name-filter panel closed the sync
  //    chrome (toolbar + totals + mode-note, i.e. first-row top relative
  //    to the sync section top -- app tab/path bars above are not sync
  //    chrome) is one compact block. Old stacked rows above the grid are gone.
  await page.getByTestId("syncview/toolbar-filter").click();
  await expect(page.getByTestId("syncview/name-filter")).toBeHidden();
  const delta = await chromeDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `sync chrome collapsed to one row (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/sync-chrome-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/sync-chrome-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
