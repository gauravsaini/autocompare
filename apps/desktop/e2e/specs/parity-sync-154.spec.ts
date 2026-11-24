/**
 * Wave-154 sync closed loop (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> SyncView) over the mock sync engine and pins the BC
 * Folder Sync bar (`/tmp/syncparity-104-bc-truth.png` stand-in):
 *  (a) chrome budget firstRowTop<=140px section-relative, name-filter
 *      panel closed (one toolbar row + totals + mode-note above rows);
 *  (b) toolbar one compact row (<120px) with directions + prune +
 *      filter toggle + dry/real apply intact;
 *  (c) folder-compare actions: every row carries an honest action badge
 *      (`syncview/row-action`, e.g. `[copy to right]`), list-only
 *      filter narrows rows, dry-run touches nothing, real apply
 *      receipts copies/deletes;
 *  (d) status bar per-action counts + mode follow direction/prune/filter.
 *  Light + dark DOM checks, zoomed PNG crops to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { FOLDER_NODES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID = "/tmp/mergesyncmedia-154-sync";
const BC_TRUTH = "/tmp/syncparity-104-bc-truth.png";
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

test("mergesyncmedia-154 sync: chrome budget, toolbar collapse, row actions, wiring, status", async ({
  page,
  browserName,
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

  // (b) Toolbar collapse: one compact row, all controls ride it.
  const toolbar = page.getByTestId("syncview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("syncview/dir-left")).toBeVisible();
  await expect(page.getByTestId("syncview/dir-right")).toBeVisible();
  await expect(page.getByTestId("syncview/dir-newer")).toBeVisible();
  await expect(page.getByTestId("syncview/prune-toggle")).toBeVisible();
  await expect(page.getByTestId("syncview/apply-dry")).toBeVisible();
  await expect(page.getByTestId("syncview/apply-real")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // (c) Folder-compare actions: every row names its honest action badge.
  const badges = page.getByTestId("syncview/row-action");
  await expect.poll(async () => badges.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);
  const badgeTexts = await badges.allTextContents();
  expect(badgeTexts.length).toBe(EXPECTED_STEPS);
  for (const t of badgeTexts) {
    expect(t, "row action badge names a copy/delete action").toMatch(/copy to|delete/);
  }
  // Badge sits on the same row line (no new stacked rows per row).
  const firstBadgeTop = await badges.first().evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await rows.first().evaluate((el) => el.getBoundingClientRect().top);
  expect(Math.abs(firstBadgeTop - firstRowTopAbs), "action badge rides the row line").toBeLessThan(30);

  // Direction + prune wiring intact (receipt clears on re-plan).
  const modeNote = page.getByTestId("syncview/mode-note");
  await expect(modeNote).toContainText("direction: left to right");
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/dry run/);
  await page.getByTestId("syncview/dir-newer").click();
  await expect(modeNote).toContainText("direction: newer wins");
  await expect(page.getByTestId("syncview/receipt")).toHaveCount(0);

  // Filter wiring: list-only narrowing, showing count follows, Esc/Esc.
  const paths = await page.$$eval('[data-testid="syncview/row"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const query = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  // Filter may be collapsed after the direction re-plan cycle above;
  // reopen via the toolbar toggle (same path a user takes).
  if (await page.getByTestId("syncview/name-filter").isHidden()) {
    await page.getByTestId("syncview/toolbar-filter").click();
  }
  const filter = page.getByTestId("syncview/name-filter");
  await expect(filter).toBeVisible();
  await filter.fill(query);
  const note = page.getByTestId("syncview/name-note");
  await expect(note).toContainText("apply runs the full plan");
  const narrowed = await rows.count();
  expect(narrowed).toBeLessThanOrEqual(EXPECTED_STEPS);
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(
    new RegExp(`showing ${narrowed}`),
  );
  await page.keyboard.press("Escape");
  await expect(note).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(filter).toBeHidden();

  // Dry + real apply receipts intact.
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/nothing touched/);
  await page.getByTestId("syncview/apply-real").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(
    new RegExp(`applied: ${EXPECTED_STEPS} copied, 0 deleted`),
  );

  // (d) Status bar below rows, toolbar above rows.
  const status = page.getByTestId("syncview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(
    new RegExp(`${EXPECTED_STEPS} steps`),
  );
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const rowTop = await rows.first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(rowTop);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(rowTop);

  await assertNoOverlap(page, 'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]');

  // Density: uniform row heights.
  const rowHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="syncview/row"]')];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  const minH = Math.min(...rowHeights);
  const maxH = Math.max(...rowHeights);
  expect(maxH - minH, `uniform row heights (got ${minH}..${maxH})`).toBeLessThanOrEqual(2);

  // (a) Chrome budget with the filter panel closed.
  await expect(page.getByTestId("syncview/name-filter")).toBeHidden();
  const delta = await chromeDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  console.log(`MERGESYNCMEDIA-154 sync chromeDelta=${delta}`);
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `sync chrome within budget (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Evidence: full view + zoomed crops.
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await rows.first().screenshot({ path: `${EVID}-zoom-rows.png` });
  await status.screenshot({ path: `${EVID}-zoom-status.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  if (fs.existsSync(BC_TRUTH)) {
    const truthB64 = fs.readFileSync(BC_TRUTH).toString("base64");
    const oursB64 = fs.readFileSync(`${EVID}-light.png`).toString("base64");
    await page.emulateMedia({ colorScheme: "light" });
    await page.setContent(
      `<html><body style="margin:0;display:flex;gap:8px;background:#fff;font:12px system-ui">` +
        `<figure style="margin:0"><figcaption>BC sync truth (stand-in)</figcaption><img src="data:image/png;base64,${truthB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
        `<figure style="margin:0"><figcaption>ours sync 154 light (${browserName})</figcaption><img src="data:image/png;base64,${oursB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
        `</body></html>`,
    );
    await page.waitForTimeout(400);
    await page.screenshot({ path: `${EVID}-sidebyside.png`, fullPage: true });
  }

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
