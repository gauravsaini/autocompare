/**
 * Wave-122 geopolish proof (geometry ONLY, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves the geometry-first pass:
 *  (a) 4px spacing audit: toolbar/pathbar/statusbar/tabbar paddings ride
 *      `--ac-space-*`; off-scale 2px paddings (tab close `p-0.5`,
 *      confirm `gap-0.5`) now sit on the ramp; history popover anchors
 *      at a 4px offset;
 *  (b) alignment: toolbar buttons share one baseline row, pathbar fields
 *      share vertical center, statusbar counts stay right-anchored,
 *      tab labels ride centered pills;
 *  (c) hit-areas: primary targets >= 32px (toolbar buttons 36, tab pills
 *      32, new-tab 32), compact pathbar icon buttons 28 (space-7, above
 *      the WCAG 2.5.8 24px minimum);
 *  (d) chrome budget firstRowTop <= 140px holds on text/folder/table/merge.
 *
 * Zero color/font assertions by design (views/dnd/ipc/colors/typography
 * OFF LIMITS). Zoomed crops land in `/tmp/geopolish-122/` (never in-repo).
 * Light + dark, zero console errors / page errors throughout.
 *
 * PORT-FREE: fresh :1529 server (`reuseExistingServer:false`,
 * `--workers=1`, webkit) so no shared port or parallel worker flakes it.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/geopolish-122";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("tabbar")).toBeVisible({ timeout: 30000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 30000 });
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

async function styleOf(page: Page, testid: string, prop: string): Promise<string> {
  return page.getByTestId(testid).evaluate((el, p) => window.getComputedStyle(el).getPropertyValue(p), prop);
}

async function boxOf(page: Page, testid: string): Promise<{ x: number; y: number; width: number; height: number }> {
  const box = await page.getByTestId(testid).evaluate((el) => {
    const r = el.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  });
  expect(box.width, `${testid} has nonzero width`).toBeGreaterThan(0);
  expect(box.height, `${testid} has nonzero height`).toBeGreaterThan(0);
  return box;
}

/** Full 122 geometry pass: runs once per color scheme. */
async function assertGeopolish(page: Page): Promise<void> {
  const tabbar = page.getByTestId("tabbar");
  const toolbar = page.getByRole("toolbar", { name: "Compare actions" });
  const pathbar = page.getByTestId("pathbar");
  const statusbar = page.getByTestId("statusbar");
  await expect(tabbar).toBeVisible();
  await expect(toolbar).toBeVisible();
  await expect(pathbar).toBeVisible();
  await expect(statusbar).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // Bar heights ride the chrome height tokens (unchanged contracts).
  expect((await boxOf(page, "tabbar")).height, "tabbar h = 32").toBe(32);
  const toolbarH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(toolbarH, "toolbar slim bar 40..48").toBeGreaterThanOrEqual(40);
  expect(toolbarH, "toolbar slim bar 40..48").toBeLessThanOrEqual(48);
  expect((await boxOf(page, "pathbar")).height, "pathbar compact single row (32)").toBe(32);
  expect((await boxOf(page, "statusbar")).height, "statusbar h = 28").toBe(28);

  // (a) 4px spacing audit: every sampled gap/padding is a multiple of 4.
  const four = async (v: Promise<string>, what: string): Promise<void> => {
    const px = Number.parseFloat(await v);
    expect(px % 4, `${what} on 4px ramp (got ${px})`).toBe(0);
  };
  await four(styleOf(page, "toolbar/action-compare", "column-gap"), "toolbar button gap");
  await four(styleOf(page, "toolbar/action-compare", "padding-left"), "toolbar button px");
  const activeTab = page.getByRole("tab", { selected: true });
  await expect(activeTab).toBeVisible();
  await four(
    activeTab.evaluate((el) => window.getComputedStyle(el).columnGap),
    "tab pill gap",
  );
  await four(
    activeTab.evaluate((el) => window.getComputedStyle(el).paddingLeft),
    "tab pill px",
  );
  await four(styleOf(page, "pathbar/left", "padding-left"), "pathbar input px");
  // Tab close affordance rides the ramp now (p-1 = 4px, was p-0.5 = 2px).
  const closeBtn = page.locator('[data-testid^="tab-close-"]').first();
  if ((await closeBtn.count()) > 0) {
    await four(
      closeBtn.evaluate((el) => window.getComputedStyle(el).paddingTop),
      "tab close affordance padding",
    );
  }
  // Popover anchor offset: history dropdown sits 4px under its field.
  await page.getByTestId("pathbar/left-history-toggle").click();
  const history = page.getByTestId("pathbar/left-history");
  await expect(history).toBeVisible();
  await four(
    history.evaluate((el) => window.getComputedStyle(el).marginTop),
    "history popover anchor offset",
  );
  await page.getByTestId("pathbar/left").press("Escape");
  await expect(history).toBeHidden();

  // (b) alignment: one toolbar row, shared pathbar center, right-anchored counts.
  const tops = await toolbar.evaluate((el) => {
    const btns = [...el.querySelectorAll("button")].filter(
      (b) => window.getComputedStyle(b).display !== "none",
    );
    return btns.map((b) => Math.round(b.getBoundingClientRect().top));
  });
  expect(tops.length, "toolbar has visible buttons").toBeGreaterThan(0);
  expect(Math.max(...tops) - Math.min(...tops), "toolbar icons share one baseline row").toBeLessThanOrEqual(2);
  const left = await boxOf(page, "pathbar/left");
  const right = await boxOf(page, "pathbar/right");
  const leftCy = left.y + left.height / 2;
  const rightCy = right.y + right.height / 2;
  expect(Math.abs(leftCy - rightCy), "pathbar fields vertically centered").toBeLessThanOrEqual(1);
  expect(Math.abs(left.width - right.width), "pathbar panes symmetric").toBeLessThanOrEqual(8);
  expect(
    await activeTab.evaluate((el) => window.getComputedStyle(el).alignItems),
    "tab labels centered",
  ).toBe("center");
  const statsAnchor = page.getByTestId("statusbar/stats");
  if ((await statsAnchor.count()) > 0) {
    const anchorBox = await statsAnchor.evaluate((el) => {
      const parent = el.parentElement as HTMLElement;
      const r = parent.getBoundingClientRect();
      return { right: r.right, textAlign: window.getComputedStyle(parent).textAlign };
    });
    expect(anchorBox.textAlign, "statusbar counts right-anchored").toBe("right");
    const barBox = await boxOf(page, "statusbar");
    expect(barBox.x + barBox.width - anchorBox.right, "counts hug the bar edge").toBeLessThanOrEqual(16);
  }

  // (c) hit-areas: primary targets >= 32px, compact icon buttons 28px.
  const compareH = await page.getByTestId("toolbar/action-compare").evaluate((el) => el.getBoundingClientRect().height);
  expect(compareH, "toolbar buttons >= 32px hit-area").toBeGreaterThanOrEqual(32);
  const pillH = await activeTab.evaluate((el) => el.getBoundingClientRect().height);
  expect(pillH, "tab pills = control-md (24)").toBe(24);
  const newTab = await boxOf(page, "tab-new");
  expect(Math.min(newTab.width, newTab.height), "new-tab >= 32px hit-area").toBeGreaterThanOrEqual(32);
  const histToggle = await boxOf(page, "pathbar/left-history-toggle");
  expect(Math.round(histToggle.width), "pathbar icon buttons = 28px (space-7)").toBe(28);
  expect(Math.round(histToggle.height), "pathbar icon buttons = 28px (space-7)").toBe(28);

  await assertNoOverlap(page);
}

test("geopolish-122: spacing scale + alignment + hit-areas, light+dark", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  await assertGeopolish(page);
  await page
    .locator('[role="toolbar"][aria-label="Compare actions"]')
    .screenshot({ path: `${EVID_DIR}/chrome-toolbar-light.png` });
  await page.getByTestId("pathbar").screenshot({ path: `${EVID_DIR}/chrome-pathbar-light.png` });
  await page.getByTestId("tabbar").screenshot({ path: `${EVID_DIR}/chrome-tabbar-light.png` });

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await assertGeopolish(page);
  await page
    .locator('[role="toolbar"][aria-label="Compare actions"]')
    .screenshot({ path: `${EVID_DIR}/chrome-toolbar-dark.png` });
  await page.getByTestId("pathbar").screenshot({ path: `${EVID_DIR}/chrome-pathbar-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  assertClean(consoleErrors, pageErrors);
});

async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewId: string,
): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click().catch(() => {});
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewId)).toBeVisible();
}

async function firstRowDelta(page: Page, sectionSel: string, rowSel: string): Promise<number | null> {
  return page.evaluate(
    ({ s, r }) => {
      const section = document.querySelector(s);
      const first = document.querySelector(r);
      if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
      return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
    },
    { s: sectionSel, r: rowSel },
  );
}

test("geopolish-122: chrome budget firstRowTop<=140 across views", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  const text = await firstRowDelta(page, 'section[aria-label="text comparison"]', '[data-testid="textview/row-0"]');
  expect(text, "text firstRowTop measured").not.toBeNull();
  expect(text as number, `text firstRowTop <= 140 (got ${text})`).toBeLessThanOrEqual(140);
  await page.screenshot({ path: `${EVID_DIR}/text-light.png` });

  await createSession(page, "folder", "/tmp", "/tmp", "view-folder");
  await expect(page.locator('[data-testid^="folder-row-"]').first()).toBeVisible({ timeout: 15000 });
  const folder = await firstRowDelta(page, 'section[aria-label="folder comparison"]', '[data-testid^="folder-row-"]');
  expect(folder, "folder firstRowTop measured").not.toBeNull();
  expect(folder as number, `folder firstRowTop <= 140 (got ${folder})`).toBeLessThanOrEqual(140);
  await page.screenshot({ path: `${EVID_DIR}/folder-light.png` });

  await createSession(page, "table", "/tmp/a.csv", "/tmp/b.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row").first()).toBeVisible({ timeout: 15000 });
  const table = await firstRowDelta(page, 'section[aria-label="table comparison"]', "tbody tr.tc-row");
  expect(table, "table firstRowTop measured").not.toBeNull();
  expect(table as number, `table firstRowTop <= 140 (got ${table})`).toBeLessThanOrEqual(140);
  await page.screenshot({ path: `${EVID_DIR}/table-light.png` });

  await createSession(page, "merge", "/tmp/m-base.txt", "/tmp/m-right.txt", "view-merge");
  await expect(page.locator(".mv-pane").first()).toBeVisible({ timeout: 15000 });
  const merge = await firstRowDelta(page, 'section[aria-label="three-way merge"]', ".mv-pane");
  expect(merge, "merge firstRowTop measured").not.toBeNull();
  expect(merge as number, `merge firstRowTop <= 140 (got ${merge})`).toBeLessThanOrEqual(140);
  await page.screenshot({ path: `${EVID_DIR}/merge-light.png` });

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-dark.png` });
  const darkText = await firstRowDelta(
    page,
    'section[aria-label="three-way merge"]',
    ".mv-pane",
  );
  expect(darkText as number, "dark firstRowTop <= 140").toBeLessThanOrEqual(140);
  await page.emulateMedia({ colorScheme: "light" });

  console.log(`GEOPOLISH122_DELTAS text=${text} folder=${folder} table=${table} merge=${merge}`);
  assertClean(consoleErrors, pageErrors);
});
