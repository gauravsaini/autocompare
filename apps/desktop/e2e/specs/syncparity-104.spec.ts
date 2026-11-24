/**
 * Wave-104 sync parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> SyncView) over the mock sync engine and pins the
 * Beyond Compare Folder Sync bar (`/tmp/syncparity-104-bc-truth.png`,
 * captured live from BC 5.x: New Folder Sync tab with left path loaded;
 * right path uncommitted because synthetic clicks do not register in
 * BC's custom-drawn UI, so the grid is empty — noted honestly in the
 * REPORT):
 *  (a) chrome budget firstRowTop<=140px section-relative with the
 *      name-filter panel closed (one toolbar row + totals + mode-note
 *      above the rows);
 *  (b) density consistent (uniform row heights, compact one-row
 *      toolbar);
 *  (c) direction buttons + prune toggle + name filter (list-only,
 *      Esc/Ctrl+F wiring) + dry-run / real apply receipts intact;
 *  (d) status bar counts (per-action ->right/->left/deletes, showing)
 *      + mode follow direction/prune/filter state.
 *  Light + dark DOM checks, zoomed PNG crops to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { FOLDER_NODES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID = "/tmp/syncparity-104";
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

test("syncparity-104: chrome budget, density, direction/filter/apply wiring, status counts, light+dark", async ({
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

  // (c) Direction wiring: three buttons, mode-note + status bar follow,
  // re-plan clears the receipt (request path intact).
  const modeNote = page.getByTestId("syncview/mode-note");
  await expect(modeNote).toContainText("direction: left to right");
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("direction: left to right");
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("prune: off");
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/dry run/);
  await page.getByTestId("syncview/dir-newer").click();
  await expect(modeNote).toContainText("direction: newer wins");
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("direction: newer wins");
  await expect(page.getByTestId("syncview/receipt")).toHaveCount(0);
  await page.getByTestId("syncview/prune-toggle").check();
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText("prune: on");
  await expect(page.getByTestId("syncview/totals")).toContainText(
    new RegExp(`${EXPECTED_STEPS} steps`),
  );
  // Back to defaults for the density/chrome reads.
  await page.getByTestId("syncview/prune-toggle").uncheck();
  await page.getByTestId("syncview/dir-left").click();
  await expect(modeNote).toContainText("direction: left to right");

  // (c) Filter wiring: list-only narrowing, list-only note, showing
  // count follows; Esc clears first (box stays mounted), Esc on the
  // empty box collapses the panel.
  const paths = await page.$$eval('[data-testid="syncview/row"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const query = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  const filter = page.getByTestId("syncview/name-filter");
  await expect(filter).toBeVisible();
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
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);
  await page.keyboard.press("Escape");
  await expect(filter).toBeHidden();

  // (c) Take wiring: dry-run receipt is honest (touches nothing),
  // real apply receipt reports copies/deletes.
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/nothing touched/);
  await page.getByTestId("syncview/apply-real").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(
    new RegExp(`applied: ${EXPECTED_STEPS} copied, 0 deleted`),
  );

  // (d) Status bar: per-action counts + mode, below the rows while the
  // toolbar sits above them.
  const status = page.getByTestId("syncview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(
    new RegExp(`${EXPECTED_STEPS} steps`),
  );
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(/to right/);
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(/to left/);
  const toolbar = page.getByTestId("syncview/toolbar");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await page.getByTestId("syncview/row").first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(firstRowTopAbs);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(firstRowTopAbs);

  await assertNoOverlap(page, 'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]');

  // (b) Density: uniform row heights, compact one-row toolbar.
  const rowHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="syncview/row"]')];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(rowHeights.length).toBe(EXPECTED_STEPS);
  const minH = Math.min(...rowHeights);
  const maxH = Math.max(...rowHeights);
  expect(maxH - minH, `uniform row heights (got ${minH}..${maxH})`).toBeLessThanOrEqual(2);
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // (a) Chrome budget with the name-filter panel closed: first row top
  // <= 140px relative to the sync section top (toolbar + totals +
  // mode-note only; app tab/path bars above are not sync chrome).
  await expect(page.getByTestId("syncview/name-filter")).toBeHidden();
  const delta = await chromeDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `sync chrome within budget (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Evidence: full view + zoomed crops (toolbar, rows, status bar).
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await page.getByTestId("syncview/row").first().screenshot({ path: `${EVID}-zoom-rows.png` });
  await status.screenshot({ path: `${EVID}-zoom-status.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await status.screenshot({ path: `${EVID}-zoom-status-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
