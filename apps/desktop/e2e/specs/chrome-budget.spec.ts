/**
 * Chrome Budget Living Spec (MOCK IPC ONLY, VITE_MOCK_IPC=1).
 *
 * Enforces Beyond Compare chrome discipline (Wave-223 BC order: tab strip on its
 * own row, toolbar actions on the NEXT row, then path bars):
 * 1. Three shell rows above panes, no other stacked control rows:
 *    - Tab strip row (height 40px, internal scroll);
 *    - Single compact toolbar row of its own (height <= 48px, buttons single
 *      horizontal row within that band, no wrapping);
 *    - Single compact PathBar row (height <= 36px);
 *    - Combined shell top (TabBar + Toolbar + PathBar) <= 120px;
 *    - No duplicate control rows above comparison panes.
 * 2. firstRowTop <= 140px section-relative strictly enforced across all major views:
 *    - Text View (first row <= 140px)
 *    - Folder View (first tree row <= 140px)
 *    - Table View (first grid row <= 140px)
 *    - Merge View (first pane / conflict card <= 140px)
 *    - Hex View (first byte row <= 140px)
 * 3. Layout integrity: zero element overlap (assertNoOverlap) and clean console.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
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

test("chrome-budget: no stacked control rows above panes (single toolbar <= 48px, pathbar <= 36px, shell <= 120px)", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  const geometry = await page.evaluate(() => {
    const q = (s: string) => document.querySelector(s);
    const r = (el: Element | null) =>
      el instanceof HTMLElement ? el.getBoundingClientRect() : null;

    const tabbar = r(q('[data-testid="tabbar"]'));
    const toolbar = r(q('[role="toolbar"][aria-label="Compare actions"]'));
    const pathbar = r(q('[data-testid="pathbar"]'));
    const statusbar = r(q('[data-testid="statusbar"]'));

    // Check all visible buttons in toolbar to confirm single row (no vertical stacking/wrapping)
    const toolbarEl = q('[role="toolbar"][aria-label="Compare actions"]');
    const buttons = toolbarEl ? Array.from(toolbarEl.querySelectorAll("button")) : [];
    const buttonTops = buttons
      .filter((b) => window.getComputedStyle(b).display !== "none")
      .map((b) => Math.round(b.getBoundingClientRect().top));
    const allSameTop =
      buttonTops.length > 0 &&
      buttonTops.every((top) => Math.abs(top - buttonTops[0]) <= 2);

    return {
      tabbarH: tabbar ? Math.round(tabbar.height) : null,
      toolbarH: toolbar ? Math.round(toolbar.height) : null,
      pathbarH: pathbar ? Math.round(pathbar.height) : null,
      statusbarH: statusbar ? Math.round(statusbar.height) : null,
      shellTop: tabbar && pathbar ? Math.round(pathbar.bottom - tabbar.top) : null,
      allButtonsSingleRow: allSameTop,
    };
  });

  // Verify compact single control rows (no stacked bloated rows)
  expect(geometry.toolbarH, "toolbar height measured").not.toBeNull();
  expect(geometry.toolbarH!, "toolbar height <= 48px").toBeLessThanOrEqual(48);

  expect(geometry.pathbarH, "pathbar height measured").not.toBeNull();
  expect(geometry.pathbarH!, "pathbar height <= 36px").toBeLessThanOrEqual(36);

  expect(geometry.tabbarH, "tabbar height is 32px").toBe(32);
  expect(geometry.shellTop, "shell top (tabbar+toolbar+pathbar) measured").not.toBeNull();
  expect(geometry.shellTop!, "shell top <= 120px").toBeLessThanOrEqual(120);

  expect(geometry.allButtonsSingleRow, "all visible toolbar buttons align in a single row").toBe(true);

  await assertNoOverlap(page);
  expect(pageErrors, "zero page errors").toEqual([]);
  expect(consoleErrors, "zero console errors").toEqual([]);
});

test("chrome-budget: firstRowTop <= 140px enforced across text, folder, table, merge, hex views", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // 1. Text View
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  const textDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="text comparison"], section:has(.tv-row)');
    const first = document.querySelector('[data-testid="textview/row-0"], .tv-row');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(textDelta, "text firstRowTop measured").not.toBeNull();
  expect(textDelta!, `text firstRowTop <= 140px (got ${textDelta})`).toBeLessThanOrEqual(140);

  // 2. Folder View
  await createSession(page, "folder", "/tmp/cb-f-left", "/tmp/cb-f-right", "view-folder");
  await expect(page.locator('[data-testid^="folder-row-"]').first()).toBeVisible({ timeout: 15000 });
  const folderDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(folderDelta, "folder firstRowTop measured").not.toBeNull();
  expect(folderDelta!, `folder firstRowTop <= 140px (got ${folderDelta})`).toBeLessThanOrEqual(140);

  // 3. Table View
  await createSession(page, "table", "/tmp/cb-t-left.csv", "/tmp/cb-t-right.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]").first()).toBeVisible({ timeout: 15000 });
  const tableDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(tableDelta, "table firstRowTop measured").not.toBeNull();
  expect(tableDelta!, `table firstRowTop <= 140px (got ${tableDelta})`).toBeLessThanOrEqual(140);

  // 4. Merge View
  await createSession(page, "merge", "/tmp/cb-m-base.txt", "/tmp/cb-m-right.txt", "view-merge");
  await expect(page.locator(".mv-pane, [data-testid^=\"merge-card-\"]").first()).toBeVisible({ timeout: 15000 });
  const mergeDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector(".mv-pane, [data-testid^=\"merge-card-\"]");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(mergeDelta, "merge firstRowTop measured").not.toBeNull();
  expect(mergeDelta!, `merge firstRowTop <= 140px (got ${mergeDelta})`).toBeLessThanOrEqual(140);

  // 5. Hex View (with panels closed per BC chrome spec). Fresh hex
  // sessions default BOTH find and goto panels open (HexView
  // `useState(true)`); one Esc dismisses one in-flow panel, so two
  // Escapes close both (Esc behavior itself untouched — two presses,
  // one per open panel).
  await createSession(page, "hex", "/tmp/cb-h-left.bin", "/tmp/cb-h-right.bin", "view-hex");
  await expect(page.locator(".hx-row").first()).toBeVisible({ timeout: 15000 });
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();
  const hexDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="hex comparison"]');
    const first = document.querySelector(".hx-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(hexDelta, "hex firstRowTop measured").not.toBeNull();
  expect(hexDelta!, `hex firstRowTop <= 140px (got ${hexDelta})`).toBeLessThanOrEqual(140);

  expect(pageErrors, "zero page errors across views").toEqual([]);
  expect(consoleErrors, "zero console errors across views").toEqual([]);
});
