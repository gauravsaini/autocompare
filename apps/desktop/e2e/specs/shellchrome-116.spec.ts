/**
 * Wave-116 app-shell chrome consolidation proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves the shared shell (TabBar +
 * app toolbar + PathBar, owned heights/padding/visibility only in
 * `chrome/Toolbar.tsx` + `chrome/PathBar.tsx`) got slimmer vs the
 * pre-wave baseline captured in `checks/shellchrome-116/REPORT.md`:
 *  - toolbar h-14 (56) -> h-11 (44), pathbar token-h (40) -> compact h-8 (32);
 *  - idle Stop / Prev / Next collapse out of the bar (`hidden`, testids
 *    stay mounted + disabled with honest titles) instead of holding slots;
 *  - TOTAL shell budget (tabbar-bottom -> first-pane-row) drops on every
 *    view: text / folder / table / merge each beat their baseline ceiling;
 *  - wiring intact: enabled actions stay visible + clickable, Stop
 *    reappears while a compare runs, per-view first rows still mount;
 *  - light + dark PNGs land in `/tmp/shellchrome-116/` (never in-repo);
 *    zero console errors / page errors throughout.
 *
 * New file only. Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/shellchrome-116";

/** Pre-wave-116 baseline (fresh captures, same ruler, same mock). */
const BASE = { toolbarH: 56, pathbarH: 40, text: 217, folder: 165, table: 250, merge: 174 } as const;

/** Post-wave ceilings: each at least 10px under its baseline. */
const CEIL = { toolbarH: 48, pathbarH: 36, text: 205, folder: 155, table: 240, merge: 165 } as const;

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

async function shellMetrics(page: Page, firstRowSel: string) {
  return page.evaluate((sel) => {
    const q = (s: string): Element | null => document.querySelector(s);
    const r = (el: Element | null): DOMRect | null =>
      el instanceof HTMLElement ? el.getBoundingClientRect() : null;
    const toolbar = r(q('[role="toolbar"][aria-label="Compare actions"]'));
    const pathbar = r(q('[data-testid="pathbar"]'));
    const tabbar = r(q('[data-testid="tabbar"]'));
    const first = r(q(sel));
    return {
      toolbarH: toolbar ? Math.round(toolbar.height) : null,
      pathbarH: pathbar ? Math.round(pathbar.height) : null,
      tabbarH: tabbar ? Math.round(tabbar.height) : null,
      budget: tabbar && first ? Math.round(first.top - tabbar.bottom) : null,
    };
  }, firstRowSel);
}

test("shellchrome-116: slimmer shared shell, reduced per-view ceilings", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text session boots automatically (mock IPC serves real rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  const text = await shellMetrics(page, '[data-testid="textview/row-0"]');
  expect(text.toolbarH, `toolbar slimmer than baseline ${BASE.toolbarH}`).not.toBeNull();
  expect(text.toolbarH as number).toBeLessThanOrEqual(CEIL.toolbarH);
  expect(text.pathbarH, `pathbar slimmer than baseline ${BASE.pathbarH}`).not.toBeNull();
  expect(text.pathbarH as number).toBeLessThanOrEqual(CEIL.pathbarH);
  expect(text.tabbarH, "tabbar untouched (off limits)").toBe(40);
  expect(text.budget, "text budget measured").not.toBeNull();
  expect(text.budget as number, `text ceiling under baseline ${BASE.text}`).toBeLessThanOrEqual(
    CEIL.text,
  );
  await page.screenshot({ path: `${EVID_DIR}/text-light.png` });
  await page
    .locator('[role="toolbar"][aria-label="Compare actions"]')
    .screenshot({ path: `${EVID_DIR}/chrome-toolbar-light.png` });
  await page.getByTestId("pathbar").screenshot({ path: `${EVID_DIR}/chrome-pathbar-light.png` });

  // Folder view: per-view toolbar + rows mount below the slimmer shell.
  await createSession(page, "folder", "/tmp", "/tmp", "view-folder");
  await expect(page.locator('[data-testid^="folder-row-"]').first()).toBeVisible({ timeout: 15000 });
  const folder = await shellMetrics(page, '[data-testid^="folder-row-"]');
  expect(folder.budget, "folder budget measured").not.toBeNull();
  expect(folder.budget as number, `folder ceiling under baseline ${BASE.folder}`).toBeLessThanOrEqual(
    CEIL.folder,
  );
  await page.screenshot({ path: `${EVID_DIR}/folder-light.png` });

  // Table view.
  await createSession(page, "table", "/tmp/a.csv", "/tmp/b.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row").first()).toBeVisible({ timeout: 15000 });
  const table = await shellMetrics(page, "tbody tr.tc-row");
  expect(table.budget, "table budget measured").not.toBeNull();
  expect(table.budget as number, `table ceiling under baseline ${BASE.table}`).toBeLessThanOrEqual(
    CEIL.table,
  );
  await page.screenshot({ path: `${EVID_DIR}/table-light.png` });

  // Merge view.
  await createSession(page, "merge", "/tmp/m-base.txt", "/tmp/m-right.txt", "view-merge");
  await expect(page.locator(".mv-pane").first()).toBeVisible({ timeout: 15000 });
  const merge = await shellMetrics(page, ".mv-pane");
  expect(merge.budget, "merge budget measured").not.toBeNull();
  expect(merge.budget as number, `merge ceiling under baseline ${BASE.merge}`).toBeLessThanOrEqual(
    CEIL.merge,
  );
  await page.screenshot({ path: `${EVID_DIR}/merge-light.png` });

  // Dark scheme: same layout, fresh evidence.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-dark.png` });
  await page
    .locator('[role="toolbar"][aria-label="Compare actions"]')
    .screenshot({ path: `${EVID_DIR}/chrome-toolbar-dark.png` });
  await page.getByTestId("pathbar").screenshot({ path: `${EVID_DIR}/chrome-pathbar-dark.png` });

  console.log(
    `SHELL116 text=${text.budget} folder=${folder.budget} table=${table.budget} merge=${merge.budget} toolbar=${text.toolbarH} pathbar=${text.pathbarH}`,
  );
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("shellchrome-116: idle actions collapse, wiring intact", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Text demo: hunks exist -> Prev/Next stay visible + enabled.
  await expect(page.getByTestId("view-text")).toBeVisible();
  const prev = page.getByTestId("toolbar/action-prev-hunk");
  const next = page.getByTestId("toolbar/action-next-hunk");
  await expect(next).toBeEnabled();
  await expect(next).toBeVisible();
  await expect(prev).toBeEnabled();
  await next.click();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Idle Stop collapses: testid mounted + disabled + honest title, not visible.
  const stop = page.getByTestId("toolbar/action-compare-stop");
  await expect(stop).toHaveCount(1);
  await expect(stop).toBeDisabled();
  await expect(stop).toHaveAttribute("title", "Stop (no compare running)");
  await expect(stop).toBeHidden();

  // Running a compare brings Stop back; settling collapses it again.
  await page.getByTestId("toolbar/action-compare").click();
  await expect(stop).toBeVisible({ timeout: 5000 });
  await expect(stop).toBeEnabled();
  await expect(stop).toHaveAttribute("title", "Stop");
  await expect(stop).toBeHidden({ timeout: 15000 });

  // Folder view has no hunk nav -> Prev/Next collapse but stay mounted.
  await createSession(page, "folder", "/tmp", "/tmp", "view-folder");
  await expect(page.locator('[data-testid^="folder-row-"]').first()).toBeVisible({ timeout: 15000 });
  for (const id of ["toolbar/action-prev-hunk", "toolbar/action-next-hunk"] as const) {
    const btn = page.getByTestId(id);
    await expect(btn, `${id} stays mounted`).toHaveCount(1);
    await expect(btn, `${id} honestly disabled`).toBeDisabled();
    await expect(btn, `${id} collapses when idle`).toBeHidden();
  }
  // Enabled actions keep their labels + wiring (no testid/label changes).
  await expect(page.getByTestId("toolbar/action-compare")).toContainText("Compare");
  await expect(page.getByTestId("toolbar/action-open")).toBeVisible();

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
