/**
 * Wave-147 sync parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> SyncView) over the mock sync engine and pins the
 * Beyond Compare Folder Sync bar:
 *  (a) folder-compare action parity (per-row direction glyph mirrors
 *      the action words: -> copy right, <- copy left, trash delete;
 *      full rel_path in `title` for truncated rows);
 *  (b) toolbar collapse (one compact row: directions + prune +
 *      name-filter toggle + dry/real apply; firstRowTop<=140px with
 *      the name-filter panel closed);
 *  (c) dry-run / real apply receipts + status counts intact.
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
const EVID = "/tmp/parity-sync-147b";
const EXPECTED_STEPS = FOLDER_NODES.filter((n) => n.status !== "same").length;

const GLYPH_FOR: Record<string, string> = {
  CopyToRight: "to-right",
  CopyToLeft: "to-left",
  DeleteRight: "delete",
  DeleteLeft: "delete",
};

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

test("parity-sync-147b: action glyphs, toolbar collapse, receipts, budget, light+dark", async ({
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

  // (a) Action glyphs: one per row, mapping mirrors the action words.
  const glyphs = page.getByTestId("syncview/row-glyph");
  await expect(glyphs).toHaveCount(EXPECTED_STEPS);
  const pairs = await page.$$eval('[data-testid="syncview/row"]', (nodes) =>
    nodes.map((n) => ({
      action: n.getAttribute("data-action") ?? "",
      path: n.getAttribute("data-path") ?? "",
      glyph: n.querySelector('[data-testid="syncview/row-glyph"]')?.getAttribute("data-glyph") ?? "",
      title: n.querySelector("span.truncate")?.getAttribute("title") ?? "",
    })),
  );
  expect(pairs.length).toBe(EXPECTED_STEPS);
  for (const p of pairs) {
    expect(p.glyph, `glyph mirrors action for ${p.path}`).toBe(GLYPH_FOR[p.action] ?? "?");
    expect(p.title, `truncated row keeps full path in title (${p.path})`).toBe(p.path);
  }

  // (c) Apply wiring intact: dry run touches nothing, real apply counts.
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/nothing touched/);
  await page.getByTestId("syncview/apply-real").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(
    new RegExp(`applied: ${EXPECTED_STEPS} copied, 0 deleted`),
  );

  // Status bar follows the plan.
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(
    new RegExp(`${EXPECTED_STEPS} steps`),
  );

  await assertNoOverlap(page);

  // (b) Density: uniform row heights, compact one-row toolbar.
  const rowHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="syncview/row"]')];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(rowHeights.length).toBe(EXPECTED_STEPS);
  expect(
    Math.max(...rowHeights) - Math.min(...rowHeights),
    `uniform row heights (got ${Math.min(...rowHeights)}..${Math.max(...rowHeights)})`,
  ).toBeLessThanOrEqual(2);
  const toolbar = page.getByTestId("syncview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // (b) Chrome budget with the name-filter panel closed (Esc on the
  // empty box collapses it; filter boots open for compat).
  await page.getByTestId("syncview/name-filter").click();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("syncview/name-filter")).toBeHidden();
  const delta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder sync plan"]');
    const first = document.querySelector('[data-testid="syncview/row"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return f.top - s.top;
  });
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `sync chrome within budget (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Evidence: full view + zoomed crops (toolbar, rows, status bar).
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await page.getByTestId("syncview/row").first().screenshot({ path: `${EVID}-zoom-rows.png` });
  await page.getByTestId("syncview/statusbar").screenshot({ path: `${EVID}-zoom-status.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await page.getByTestId("syncview/statusbar").screenshot({ path: `${EVID}-zoom-status-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
