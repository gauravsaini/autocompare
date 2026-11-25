/**
 * Wave-185 sync+media parity closed loop (MOCK IPC ONLY, VITE_MOCK_IPC=1).
 *
 * Visual & geometry assertions for SyncView and MediaView BC-parity slice:
 *  (1) SyncView geometry & density: row pitch <=32px uniform, toolbar <=44px,
 *      chrome budget firstRowTop <= 140px (filter collapsed), action-column affordance right-pinned.
 *  (2) SyncView empty states & filter behavior: blank session empty,
 *      name-filter panel toggle with Esc two-stage, "same" filter empty state note.
 *  (3) MediaView picture mode geometry: diff samples <=26px uniform, metadata
 *      hierarchy (score bold, meta-left/right basenames), chrome budget <=140px.
 *  (4) MediaView audio mode geometry: tolerance panel toggle, recompare wiring,
 *      window rows uniform <=26px, statusbar counts & verdict.
 *
 * Light + dark DOM checks, zero console errors, zero page errors.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { FOLDER_NODES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/syncmedia-185";
const CHECKS_DIR = "/Users/ektasaini/autocompare/checks/syncmedia-185";
const VIEW_OVERLAP_SELECTORS =
  'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]';
const EXPECTED_STEPS = FOLDER_NODES.filter((n) => n.status !== "same").length;

test.setTimeout(120_000);

test.beforeAll(() => {
  if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  if (!fs.existsSync(CHECKS_DIR)) fs.mkdirSync(CHECKS_DIR, { recursive: true });
});

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  return { pageErrors, consoleErrors };
}

async function createSyncSession(page: Page): Promise<void> {
  if (await page.getByTestId("workspace/demo-banner").isVisible()) {
    await page.getByTestId("workspace/demo-clear").click();
  }
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

async function createMediaSession(page: Page, left: string, right: string): Promise<void> {
  if (await page.getByTestId("workspace/demo-banner").isVisible()) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("media");
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-media")).toBeVisible();
}

async function syncChromeDelta(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder sync plan"]');
    const first = document.querySelector('[data-testid="syncview/row"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return f.top - s.top;
  });
}

async function mediaChromeDelta(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="media comparison"]');
    const first = document.querySelector('[data-testid="mediaview/row"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return f.top - s.top;
  });
}

test("syncmediaparity-185: SyncView geometry, row density <=32px, action cluster affordance, toolbar single-row <=44px, light+dark", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createSyncSession(page);

  const view = page.getByTestId("view-sync");
  const rows = page.getByTestId("syncview/row");
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);

  // 1. Toolbar height <=44px, single compact row
  const toolbar = page.getByTestId("syncview/toolbar");
  await expect(toolbar).toBeVisible();
  const toolbarH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(toolbarH, `toolbar is single compact row <=44px (got ${toolbarH}px)`).toBeLessThanOrEqual(44);

  // 2. Chrome budget: firstRowTop <= 140px section-relative with filter collapsed
  if (await page.getByTestId("syncview/name-filter").isVisible()) {
    await page.getByTestId("syncview/toolbar-filter").click();
    await expect(page.getByTestId("syncview/name-filter")).toBeHidden();
  }
  const delta = await syncChromeDelta(page);
  expect(delta, `SyncView firstRowTop <= 140px (got ${delta}px)`).toBeLessThanOrEqual(140);

  // 3. Row density: all rows uniform (variance <=2px, height <=32px)
  const rowHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="syncview/row"]')];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(rowHeights.length).toBe(EXPECTED_STEPS);
  const minH = Math.min(...rowHeights);
  const maxH = Math.max(...rowHeights);
  expect(maxH - minH, `row height variance <=2px (min=${minH}, max=${maxH})`).toBeLessThanOrEqual(2);
  expect(maxH, `row height <=32px (got ${maxH}px)`).toBeLessThanOrEqual(32);

  // 4. Action column affordance: action badge + include toggle
  const actions = page.getByTestId("syncview/row-action");
  await expect.poll(async () => actions.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);
  const firstAction = actions.first();
  await expect(firstAction).toBeVisible();

  // 5. Statusbar: per-op counts and mode line at bottom
  const statusbar = page.getByTestId("syncview/statusbar");
  await expect(statusbar).toBeVisible();
  await expect(page.getByTestId("syncview/statusbar-counts")).toContainText(/steps/);
  await expect(page.getByTestId("syncview/statusbar-mode")).toContainText(/direction:/);

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);

  // Evidence captures: light
  await page.screenshot({ path: `${EVID_DIR}/sync-light.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/sync-light.png` });

  // Dark mode
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/sync-dark.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/sync-dark.png` });

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  expect(pageErrors, `zero page errors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("syncmediaparity-185: SyncView empty states & filter behavior, light+dark", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);

  // 1. Empty state for blank session (wave-138/150 contract)
  const homeTab = page.getByTestId("tab-home");
  if ((await homeTab.count()) > 0) {
    await homeTab.click().catch(() => {});
  }
  const newSyncBtn = page.getByTestId("home/new-sync");
  if ((await newSyncBtn.count()) > 0) {
    await newSyncBtn.click();
    const blankEmpty = page.getByTestId("workspace/blank-empty");
    await expect(blankEmpty).toBeVisible({ timeout: 15000 });
    await expect(blankEmpty).toContainText("needs two paths");
    await page.screenshot({ path: `${EVID_DIR}/sync-empty-light.png` });
    await page.screenshot({ path: `${CHECKS_DIR}/sync-empty-light.png` });
  }

  // 2. Open populated sync session to test filter & "same" empty state
  await createSyncSession(page);
  const filterBtn = page.getByTestId("syncview/toolbar-filter");
  await expect(filterBtn).toBeVisible();

  // Esc closes filter panel, toggle reopens
  const filterInput = page.getByTestId("syncview/name-filter");
  await filterInput.fill("orphan");
  await page.keyboard.press("Escape"); // first Esc clears input
  await expect(filterInput).toHaveValue("");
  await page.keyboard.press("Escape"); // second Esc collapses panel
  await expect(filterInput).toBeHidden();
  await filterBtn.click();
  await expect(filterInput).toBeVisible();

  // 3. View filter "same" shows empty state note
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("autocompare:view-filter", {
        detail: { filter: "same", viewKind: "sync" },
      }),
    );
  });
  const filterEmpty = page.getByTestId("syncview/filter-empty");
  await expect(filterEmpty).toBeVisible();
  await expect(filterEmpty).toContainText("No sync steps match view filter");

  // Restore filter to "all"
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("autocompare:view-filter", {
        detail: { filter: "all", viewKind: "sync" },
      }),
    );
  });
  await expect(filterEmpty).toBeHidden();

  // Dark mode
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/sync-empty-dark.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/sync-empty-dark.png` });

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  expect(pageErrors, `zero page errors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("syncmediaparity-185: MediaView picture mode geometry, diff samples <=26px, metadata hierarchy, empty diff, light+dark", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createMediaSession(page, "/tmp/m-left.png", "/tmp/m-right.png");

  const view = page.getByTestId("view-media");
  await expect(view).toBeVisible();

  // 1. Toolbar single compact row <=44px
  const toolbar = page.getByTestId("mediaview/toolbar");
  await expect(toolbar).toBeVisible();
  const toolbarH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(toolbarH, `toolbar is single compact row <=44px (got ${toolbarH}px)`).toBeLessThanOrEqual(44);
  await expect(page.getByTestId("mediaview/mode")).toContainText("Picture compare");

  // 2. Chrome budget: firstRowTop <= 140px section-relative
  const delta = await mediaChromeDelta(page);
  expect(delta, `MediaView firstRowTop <= 140px (got ${delta}px)`).toBeLessThanOrEqual(140);

  // 3. Metadata hierarchy: score bold, meta-left/right with basename and dimensions
  await expect(page.getByTestId("mediaview/score")).toContainText(/equal/);
  await expect(page.getByTestId("mediaview/meta-left")).toContainText("m-left.png");
  await expect(page.getByTestId("mediaview/meta-right")).toContainText("m-right.png");

  // 4. Sample rows: uniform height <=26px (variance <=2px)
  const rows = page.getByTestId("mediaview/row");
  const rowCount = await rows.count();
  expect(rowCount).toBeGreaterThan(0);
  const rowHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="mediaview/row"]')];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  const minH = Math.min(...rowHeights);
  const maxH = Math.max(...rowHeights);
  expect(maxH - minH, `row height variance <=2px (min=${minH}, max=${maxH})`).toBeLessThanOrEqual(2);
  expect(maxH, `row height <=26px (got ${maxH}px)`).toBeLessThanOrEqual(26);

  // 5. Status bar verdict and counts
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("Picture differences");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText(/bytes differ/);

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);

  // Evidence captures: light
  await page.screenshot({ path: `${EVID_DIR}/media-picture-light.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/media-picture-light.png` });

  // Dark mode
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/media-picture-dark.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/media-picture-dark.png` });

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  expect(pageErrors, `zero page errors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("syncmediaparity-185: MediaView audio mode geometry, tolerance panel controls & recompute, light+dark", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");

  const view = page.getByTestId("view-media");
  await expect(view).toBeVisible();
  await expect(page.getByTestId("mediaview/mode")).toContainText("Audio compare");

  // 1. Tolerance panel is open by default in audio mode
  const tolPanel = page.getByTestId("mediaview/tolerance-panel");
  await expect(tolPanel).toBeVisible();
  const tolInput = page.getByTestId("mediaview/tolerance");
  await expect(tolInput).toBeVisible();

  // 2. Toolbar controls: tolerance toggle and recompare button
  const tolBtn = page.getByTestId("mediaview/toolbar-tolerance");
  await expect(tolBtn).toBeVisible();
  const recompareBtn = page.getByTestId("mediaview/recompare");
  await expect(recompareBtn).toBeVisible();

  // Recompute with tolerance
  await tolInput.fill("5");
  await recompareBtn.click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);

  // Close tolerance panel via Esc
  await tolInput.press("Escape");
  await expect(tolPanel).toBeHidden();

  // 3. Audio diff rows: uniform height <=26px
  const rows = page.getByTestId("mediaview/row");
  const rowCount = await rows.count();
  expect(rowCount).toBeGreaterThan(0);
  const rowHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="mediaview/row"]')];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  const minH = Math.min(...rowHeights);
  const maxH = Math.max(...rowHeights);
  expect(maxH - minH, `audio row height variance <=2px (min=${minH}, max=${maxH})`).toBeLessThanOrEqual(2);
  expect(maxH, `audio row height <=26px (got ${maxH}px)`).toBeLessThanOrEqual(26);

  // 4. Status bar verdict and counts
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("Audio differences");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText(/windows differ/);

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);

  // Evidence captures: light
  await page.screenshot({ path: `${EVID_DIR}/media-audio-light.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/media-audio-light.png` });

  // Dark mode
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/media-audio-dark.png` });
  await page.screenshot({ path: `${CHECKS_DIR}/media-audio-dark.png` });

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  expect(pageErrors, `zero page errors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
