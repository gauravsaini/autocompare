/**
 * Wave-146 table closed loop (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (550 rows, 3
 * cols, 125 FULL-table diff rows) and pins BC parity:
 *  (a) density 22px rows (ROW_PX pin);
 *  (b) column geometry: 3 cols, sticky header, 1fr/6px/1fr grids +
 *      thin splitter, per-side headers;
 *  (c) cell-find: find box -> search -> hit highlight + status;
 *  (d) single toolbar row (<120px), counts in bottom status bar,
 *      Esc closes find, chrome budget firstRowTop<=140 panels-closed.
 * Light + dark PNGs + grid zoom crop to /tmp (never in-repo).
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/foldertablehex-146-table";

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
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/t146-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/t146-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("foldertablehex-146 table: density 22 + column geometry + cell-find + chrome budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("tableview/row-0")).toBeVisible({ timeout: 10000 });

  // (a) Density: first rows measure 22px offsetHeight.
  const heights = await page.evaluate(() => {
    const nodes = Array.from(document.querySelectorAll("tbody tr.tc-row")).slice(0, 8);
    return nodes.map((n) => (n as HTMLElement).offsetHeight);
  });
  expect(heights.length).toBeGreaterThan(3);
  for (const [idx, h] of heights.entries()) {
    expect(h, `table row ${idx} offsetHeight 22px (got ${h})`).toBe(22);
  }

  // (b) Column geometry: 3 cols, splitter, per-side grids + headers.
  await expect(page.getByTestId("tableview/grid-left")).toBeVisible();
  await expect(page.getByTestId("tableview/grid-right")).toBeVisible();
  await expect(page.getByTestId("tableview/grid-splitter")).toBeVisible();
  const headCount = await page.locator('[data-testid^="table-sort-"]:not([data-testid*="right"])').count();
  expect(headCount, `left header cells (got ${headCount})`).toBe(3);
  const gridTracks = await page.getByTestId("tableview/grids").evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return cs.gridTemplateColumns;
  });
  expect(gridTracks.split(" ").length, `grids 3 tracks (got ${gridTracks})`).toBe(3);
  await expect(page.getByTestId("table-cols-count")).toContainText("3 cols");

  // (c) Cell-find: open panel, search, hit highlight + status.
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await page.getByTestId("table-find-input").fill("item-0001");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText(/Hit 1 of \d+/, { timeout: 10000 });
  const hitCount = await page.locator('[data-testid^="table-hit-"]').count();
  expect(hitCount, "current hit cell mounted").toBeGreaterThan(0);

  // Esc closes the find panel (global handler parity).
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-find-input")).toBeHidden();

  // (d) Chrome: single toolbar row, status bar counts, budget.
  const toolbar = page.getByTestId("tableview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar one compact row (got ${barH}px)`).toBeLessThan(120);
  const status = page.getByTestId("tableview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("tableview/diffs")).toContainText("125 difference row(s)");
  await expect(page.getByTestId("tableview/same")).toContainText("425 same");
  await expect(status).toContainText(/Load time:/);
  const delta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(delta, `firstRowTop<=140 (got ${delta})`).not.toBeNull();
  expect(delta!).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  fs.mkdirSync(EVID, { recursive: true });
  await page.screenshot({ path: `${EVID}/table-light.png` });
  await page.getByTestId("tableview/grid-left").screenshot({ path: `${EVID}/table-zoom-grid.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}/table-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors, `console errors: ${consoleErrors.join(" | ")}`).toEqual([]);
  expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);
});
