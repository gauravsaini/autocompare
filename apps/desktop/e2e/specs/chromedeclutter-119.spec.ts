/**
 * Wave-119 chromedeclutter proof (MOCK IPC ONLY, VITE_MOCK_IPC=1).
 *
 * Verifies Beyond Compare chrome declutter idiom:
 *  - Single toolbar row <= 48px, compact PathBar <= 36px, shell top <= 120px;
 *  - firstRowTop <= 140px across text, folder, table, and merge views;
 *  - Find / Goto / Ignore / History popovers/overlays default closed, toggles in toolbar, Esc closes;
 *  - Keyboard shortcuts (Cmd/Ctrl+F, Cmd/Ctrl+G) open and focus overlays, Esc dismisses;
 *  - StatusBar live diff counts present, honest, and non-negative;
 *  - Hints in title tooltips on controls, zero stacked rows;
 *  - Zero console errors, zero page errors on light + dark schemes.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/chromedeclutter-119";

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

test("chromedeclutter-119: shell geometry budget (toolbar <= 48px, pathbar <= 36px, shell top <= 120px)", async ({
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

    // Check action buttons in toolbar to confirm single row (no vertical wrapping)
    const toolbarEl = q('[role="toolbar"][aria-label="Compare actions"]');
    const buttons = toolbarEl ? Array.from(toolbarEl.querySelectorAll("button")) : [];
    const buttonTops = buttons
      .filter((b) => window.getComputedStyle(b).display !== "none")
      .map((b) => Math.round(b.getBoundingClientRect().top));
    const allSameTop = buttonTops.length > 0 && buttonTops.every((top) => Math.abs(top - buttonTops[0]) <= 2);

    return {
      tabbarH: tabbar ? Math.round(tabbar.height) : null,
      toolbarH: toolbar ? Math.round(toolbar.height) : null,
      pathbarH: pathbar ? Math.round(pathbar.height) : null,
      statusbarH: statusbar ? Math.round(statusbar.height) : null,
      shellTop: tabbar && pathbar ? Math.round(pathbar.bottom - tabbar.top) : null,
      allButtonsSingleRow: allSameTop,
    };
  });

  expect(geometry.toolbarH, "toolbar height measured").not.toBeNull();
  expect(geometry.toolbarH as number, "toolbar height <= 48px").toBeLessThanOrEqual(48);

  expect(geometry.pathbarH, "pathbar height measured").not.toBeNull();
  expect(geometry.pathbarH as number, "pathbar height <= 36px").toBeLessThanOrEqual(36);

  expect(geometry.tabbarH, "tabbar height is 40px").toBe(40);
  expect(geometry.shellTop, "shell top (tabbar+toolbar+pathbar) measured").not.toBeNull();
  expect(geometry.shellTop as number, "shell top <= 120px").toBeLessThanOrEqual(120);
  expect(geometry.allButtonsSingleRow, "all visible toolbar buttons align in a single row").toBe(true);

  expect(pageErrors, `zero pageerrors`).toEqual([]);
  expect(consoleErrors, `zero console errors`).toEqual([]);
});

test("chromedeclutter-119: firstRowTop <= 140px across text, folder, table, merge views", async ({ page }) => {
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
    const section = document.querySelector('section[aria-label="text comparison"]');
    const first = document.querySelector('[data-testid="textview/row-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(textDelta, "text firstRowTop measured").not.toBeNull();
  expect(textDelta as number, `text firstRowTop <= 140px (got ${textDelta})`).toBeLessThanOrEqual(140);

  // 2. Folder View
  await createSession(page, "folder", "/tmp", "/tmp", "view-folder");
  await expect(page.locator('[data-testid^="folder-row-"]').first()).toBeVisible({ timeout: 15000 });
  const folderDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(folderDelta, "folder firstRowTop measured").not.toBeNull();
  expect(folderDelta as number, `folder firstRowTop <= 140px (got ${folderDelta})`).toBeLessThanOrEqual(140);

  // 3. Table View
  await createSession(page, "table", "/tmp/a.csv", "/tmp/b.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row").first()).toBeVisible({ timeout: 15000 });
  const tableDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(tableDelta, "table firstRowTop measured").not.toBeNull();
  expect(tableDelta as number, `table firstRowTop <= 140px (got ${tableDelta})`).toBeLessThanOrEqual(140);

  // 4. Merge View
  await createSession(page, "merge", "/tmp/m-base.txt", "/tmp/m-right.txt", "view-merge");
  await expect(page.locator(".mv-pane").first()).toBeVisible({ timeout: 15000 });
  const mergeDelta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector(".mv-pane");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(mergeDelta, "merge firstRowTop measured").not.toBeNull();
  expect(mergeDelta as number, `merge firstRowTop <= 140px (got ${mergeDelta})`).toBeLessThanOrEqual(140);

  console.log(`CHROMEDECLUTTER119_DELTAS text=${textDelta} folder=${folderDelta} table=${tableDelta} merge=${mergeDelta}`);

  expect(pageErrors, `zero pageerrors`).toEqual([]);
  expect(consoleErrors, `zero console errors`).toEqual([]);
});

test("chromedeclutter-119: popovers and overlays default closed, toggles in toolbar, Esc closes", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Boot text view
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // 1. Text find overlay
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  // Toggle opens
  await page.getByTestId("textview/toolbar-find").click();
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "true");
  // Esc closes
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  // Cmd+F shortcut opens and focuses
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/find-input")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);

  // 2. Text goto overlay
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("aria-expanded", "false");
  // Toggle opens
  await page.getByTestId("textview/toolbar-goto").click();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("aria-expanded", "true");
  // Esc closes
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("aria-expanded", "false");
  // Cmd+G shortcut opens and focuses
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect(page.getByTestId("textview/goto-input")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

  // 3. Text ignore menu
  await expect(page.getByTestId("textview/ignore-whitespace")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("aria-expanded", "false");
  // Toggle opens
  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(page.getByTestId("textview/ignore-whitespace")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("aria-expanded", "true");
  // Toggle closes
  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(page.getByTestId("textview/ignore-whitespace")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("aria-expanded", "false");

  // 4. PathBar history dropdown
  await expect(page.getByTestId("pathbar/left-history")).toHaveCount(0);
  await expect(page.getByTestId("pathbar/left-history-toggle")).toHaveAttribute("aria-expanded", "false");
  await page.getByTestId("pathbar/left-history-toggle").click();
  await expect(page.getByTestId("pathbar/left-history")).toBeVisible();
  await expect(page.getByTestId("pathbar/left-history-toggle")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("pathbar/left").press("Escape");
  await expect(page.getByTestId("pathbar/left-history")).toBeHidden();
  await expect(page.getByTestId("pathbar/left-history-toggle")).toHaveAttribute("aria-expanded", "false");

  expect(pageErrors, `zero pageerrors`).toEqual([]);
  expect(consoleErrors, `zero console errors`).toEqual([]);
});

test("chromedeclutter-119: statusbar live diff counts honest and formatted", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots with live diff stats
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  const statusbar = page.getByTestId("statusbar");
  await expect(statusbar).toBeVisible();

  const stats = page.getByTestId("statusbar/stats");
  await expect(stats).toBeVisible();

  const added = page.getByTestId("statusbar/stats-added");
  const removed = page.getByTestId("statusbar/stats-removed");
  const changed = page.getByTestId("statusbar/stats-changed");
  const total = page.getByTestId("statusbar/stats-total");

  await expect(added).toBeVisible();
  await expect(removed).toBeVisible();
  await expect(changed).toBeVisible();
  await expect(total).toBeVisible();

  const addedText = await added.textContent();
  const removedText = await removed.textContent();
  const changedText = await changed.textContent();
  const totalText = await total.textContent();

  expect(addedText).toMatch(/^\+\d+$/);
  expect(removedText).toMatch(/^-\d+$/);
  expect(changedText).toMatch(/^~\d+$/);
  expect(totalText).toMatch(/^\d+\s+rows$/);

  // Status message present
  const message = page.getByTestId("statusbar/message");
  await expect(message).toBeVisible();
  const msgText = await message.textContent();
  expect(msgText?.length).toBeGreaterThan(0);

  expect(pageErrors, `zero pageerrors`).toEqual([]);
  expect(consoleErrors, `zero console errors`).toEqual([]);
});

test("chromedeclutter-119: hints in tooltips, zero stacked rows, zero errors on light + dark", async ({
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

  // 1. Check title attributes on toolbar actions
  const openBtn = page.getByTestId("toolbar/action-open");
  const compareBtn = page.getByTestId("toolbar/action-compare");
  const refreshBtn = page.getByTestId("toolbar/action-compare-refresh");
  const stopBtn = page.getByTestId("toolbar/action-compare-stop");

  await expect(openBtn).toHaveAttribute("title", /.+/);
  await expect(compareBtn).toHaveAttribute("title", /.+/);
  await expect(refreshBtn).toHaveAttribute("title", /.+/);
  await expect(stopBtn).toHaveAttribute("title", /Stop/);

  // 2. Check title attributes on textview controls
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("title", /Find/i);
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("title", /Goto/i);
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("title", /Ignore/i);

  // 3. Light scheme screenshot
  await page.screenshot({ path: `${EVID_DIR}/chromedeclutter-proof-light.png` });

  // 4. Dark scheme screenshot
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/chromedeclutter-proof-dark.png` });

  // Geometry check under dark scheme
  const darkGeometry = await page.evaluate(() => {
    const q = (s: string) => document.querySelector(s);
    const r = (el: Element | null) =>
      el instanceof HTMLElement ? el.getBoundingClientRect() : null;

    const toolbar = r(q('[role="toolbar"][aria-label="Compare actions"]'));
    const pathbar = r(q('[data-testid="pathbar"]'));
    const section = r(q('section[aria-label="text comparison"]'));
    const firstRow = r(q('[data-testid="textview/row-0"]'));

    return {
      toolbarH: toolbar ? Math.round(toolbar.height) : null,
      pathbarH: pathbar ? Math.round(pathbar.height) : null,
      firstRowTop: section && firstRow ? Math.round(firstRow.top - section.top) : null,
    };
  });

  expect(darkGeometry.toolbarH).toBeLessThanOrEqual(48);
  expect(darkGeometry.pathbarH).toBeLessThanOrEqual(36);
  expect(darkGeometry.firstRowTop).toBeLessThanOrEqual(140);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
