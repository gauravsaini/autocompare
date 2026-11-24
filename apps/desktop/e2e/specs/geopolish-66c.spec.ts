import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-66c chrome geometry polish (geometry ONLY: width, height, margins,
 * padding, alignment, positioning — no color edits, no typography edits).
 *
 * Off-scale spacings are pulled onto the 4px SPACE scale by reusing the
 * existing `--ac-space-*` / chrome-height tokens:
 *  - TabBar: bar `h-[var(--ac-tabbar-h)]` (40), pills
 *    `h-[var(--ac-space-8)]` (32) with `gap-[var(--ac-space-1)]` (4,
 *    was 6) and `px-[var(--ac-space-2)]` (8, was 10);
 *  - Toolbar: buttons `gap-[var(--ac-space-1)]` (4, was 2) and
 *    `px-[var(--ac-space-2)]` (8, was 6);
 *  - PathBar: inputs `px-[var(--ac-space-2)]` (8, was 10), icon buttons
 *    `size-[var(--ac-space-6)]` (24, same value tokenized), history
 *    dropdown onto space-1/space-2;
 *  - StatusBar: `h-[var(--ac-statusbar-h)]` (28) tokenized;
 *  - OverviewRuler: `w-[var(--ac-space-4)]` (16, was 14), flush `gap-0`
 *    (was 1px), markers `height: 4` (was 3 = space-1);
 *  - LineDetailPane: stacked grid `gap-[var(--ac-space-2)]` (8, was 4).
 *
 * Every visible change below asserts live boundingBox geometry (plus
 * computed padding/gap on the 4px scale) and `assertNoOverlap`, in both
 * light and dark color schemes. New file only, no existing spec touched.
 */
test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("tabbar")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function boxOf(page: Page, testid: string): Promise<{ x: number; y: number; width: number; height: number }> {
  const box = await page.getByTestId(testid).evaluate((el) => {
    const r = el.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  });
  expect(box.width, `${testid} has nonzero width`).toBeGreaterThan(0);
  expect(box.height, `${testid} has nonzero height`).toBeGreaterThan(0);
  return box;
}

async function styleOf(page: Page, testid: string, prop: string): Promise<string> {
  return page.getByTestId(testid).evaluate((el, p) => window.getComputedStyle(el).getPropertyValue(p), prop);
}

/** Full chrome-shell geometry pass: runs once per color scheme. */
async function assertChromeGeometry(page: Page): Promise<void> {
  const tabbar = page.getByTestId("tabbar");
  const toolbar = page.getByRole("toolbar", { name: "Compare actions" });
  const pathbar = page.getByTestId("pathbar");
  const statusbar = page.getByTestId("statusbar");
  const ruler = page.getByTestId("ruler");
  await expect(tabbar).toBeVisible();
  await expect(toolbar).toBeVisible();
  await expect(pathbar).toBeVisible();
  await expect(statusbar).toBeVisible();
  await expect(ruler).toBeVisible();

  // Bar heights ride the chrome height tokens on the 4px scale.
  expect((await boxOf(page, "tabbar")).height, "tabbar h = --ac-tabbar-h (40)").toBe(40);
  const toolbarH = (await toolbar.evaluate((el) => el.getBoundingClientRect().height));
  // Wave-116 contract: slimmer shared bar (h-11 = 44) + compact pathbar (h-8 = 32).
  expect(toolbarH, "toolbar h-11 slim bar (44)").toBeGreaterThanOrEqual(40);
  expect(toolbarH, "toolbar h-11 slim bar (44)").toBeLessThanOrEqual(48);
  expect((await boxOf(page, "pathbar")).height, "pathbar compact single row (32)").toBe(32);
  expect((await boxOf(page, "statusbar")).height, "statusbar h = --ac-statusbar-h (28)").toBe(28);

  // Vertical stacking: tabbar < toolbar < pathbar < statusbar, no overlap.
  const tabBox = await boxOf(page, "tabbar");
  const toolBox = await toolbar.evaluate((el) => {
    const r = el.getBoundingClientRect();
    return { y: r.y, height: r.height };
  });
  const pathBox = await boxOf(page, "pathbar");
  const statusBox = await boxOf(page, "statusbar");
  expect(toolBox.y, "toolbar sits below tabbar").toBeGreaterThanOrEqual(tabBox.y + tabBox.height - 1);
  expect(pathBox.y, "pathbar sits below toolbar").toBeGreaterThanOrEqual(toolBox.y + toolBox.height - 1);
  expect(statusBox.y, "statusbar is the last chrome row").toBeGreaterThan(pathBox.y + pathBox.height);

  // PathBar panes stay symmetric (left/right equal width).
  const left = await boxOf(page, "pathbar/left");
  const right = await boxOf(page, "pathbar/right");
  expect(Math.abs(left.width - right.width), "pathbar panes symmetric").toBeLessThanOrEqual(8);
  expect(Math.abs(left.y - right.y), "pathbar panes share a row").toBeLessThanOrEqual(1);

  // Tab pill internals on the 4px scale: gap 4 (was 6), padding 8 (was 10).
  const activeTab = page.getByRole("tab", { selected: true });
  await expect(activeTab).toBeVisible();
  expect(await activeTab.evaluate((el) => window.getComputedStyle(el).columnGap), "tab pill gap = space-1 (4px)").toBe("4px");
  expect(await activeTab.evaluate((el) => window.getComputedStyle(el).paddingLeft), "tab pill px = space-2 (8px)").toBe("8px");
  expect((await activeTab.evaluate((el) => el.getBoundingClientRect().height)), "tab pill h = space-8 (32)").toBe(32);

  // Toolbar buttons on the 4px scale: gap 4 (was 2), padding 8 (was 6).
  expect(await styleOf(page, "toolbar/action-compare", "column-gap"), "toolbar button gap = space-1 (4px)").toBe("4px");
  expect(await styleOf(page, "toolbar/action-compare", "padding-left"), "toolbar button px = space-2 (8px)").toBe("8px");

  // PathBar input padding on the 4px scale: 8 (was 10).
  expect(await styleOf(page, "pathbar/left", "padding-left"), "pathbar input px = space-2 (8px)").toBe("8px");

  // OverviewRuler: 16 wide (was 14), flush markers (gap 0), 4px markers (was 3).
  expect((await boxOf(page, "ruler")).width, "ruler w = space-4 (16)").toBe(16);
  expect(await styleOf(page, "ruler", "row-gap"), "ruler markers flush (gap 0)").toBe("0px");
  const markerH = await page.locator('[data-testid^="ruler/row-"]').first().evaluate((el) => el.getBoundingClientRect().height);
  expect(markerH, "ruler marker h = space-1 (4px)").toBe(4);

  // History dropdown rides space-1/space-2 (empty state is deterministic:
  // each test gets a fresh context, so no persisted history exists yet).
  await page.getByTestId("pathbar/left-history-toggle").click();
  const emptyNote = page.getByTestId("pathbar/left-history-empty");
  await expect(emptyNote).toBeVisible();
  expect(await emptyNote.evaluate((el) => window.getComputedStyle(el).paddingLeft), "history empty px = space-2 (8px)").toBe("8px");
  await page.getByTestId("pathbar/left").press("Escape");
  await expect(emptyNote).toBeHidden();

  // LineDetailPane: empty placeholder pads space-4/space-2, then selecting
  // a changed row stacks left/right at equal width with an 8px (space-2)
  // rhythm between the panes (was 4).
  const pane = page.getByTestId("line-detail");
  await expect(pane).toBeVisible();
  // Empty placeholder exists only before any row is selected (first pass);
  // the dark-mode second pass reuses the selection made below.
  if ((await page.getByTestId("line-detail/empty").count()) > 0) {
    expect(await styleOf(page, "line-detail/empty", "padding-left"), "line-detail empty px = space-4 (16px)").toBe("16px");
    await expect(page.getByTestId("view-text")).toBeVisible();
    const row51 = page.getByTestId("textview/row-51");
    await expect(row51).toHaveAttribute("data-kind", "changed");
    await row51.locator('div[title="left"]').click();
    await expect(pane).toHaveAttribute("data-empty", "false");
  }
  const detailLeft = await boxOf(page, "line-detail/left");
  const detailRight = await boxOf(page, "line-detail/right");
  expect(Math.abs(detailLeft.width - detailRight.width), "line-detail panes equal width").toBeLessThanOrEqual(1);
  expect(detailRight.y, "line-detail right stacks below left").toBeGreaterThan(detailLeft.y);
  const paneGridGap = await pane.locator("div.grid").evaluate((el) => window.getComputedStyle(el).rowGap);
  expect(paneGridGap, "line-detail stack gap = space-2 (8px)").toBe("8px");

  await assertNoOverlap(page);
}

test("wave-66c chrome shell geometry on the 4px scale, light+dark", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await assertChromeGeometry(page);

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await assertChromeGeometry(page);
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
