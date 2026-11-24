/**
 * Wave-78 folder subdirectory descend proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> folder tab)
 * and proves Beyond Compare style subdirectory descend:
 *  - directory-row activation (Enter or double-click) re-roots the folder
 *    compare into that subdirectory;
 *  - rows re-root: row paths display relative to the subdirectory and only
 *    its descendants are rendered;
 *  - up-level button (`folder-up-level`) and breadcrumbs return to parent/root;
 *  - root shows no up-level;
 *  - one-sided (orphan) directory rows render an honest inline note
 *    (`folder-open-note`), never crash, and do not re-root;
 *  - collapsed chrome budget: firstRowTop <= 140px intact;
 *  - light + dark PNGs land in `/tmp/ui-78/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file only;
 * no existing spec touched.
 */
import { mkdirSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-78";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createFolderSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();
}

test("folder descend: dblclick/enter re-roots into subdir, up-level returns, orphan dir notes honestly, budget intact, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createFolderSession(page);
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // 1. Root shows no up-level button or breadcrumb bar.
  await expect(page.getByTestId("folder-up-level")).toHaveCount(0);
  await expect(page.getByTestId("folder-breadcrumb")).toHaveCount(0);

  // 2. Orphan directory (d00: leftOnly) double-click shows honest note,
  //    never crashes, and does not re-root.
  const d00Row = page.locator('[data-testid^="folder-row-"][data-path="d00"]');
  await expect(d00Row).toBeVisible();
  await expect(d00Row).toHaveAttribute("data-status", "leftOnly");
  await d00Row.dblclick();

  const openNote = page.getByTestId("folder-open-note");
  await expect(openNote).toBeVisible();
  await expect(openNote).toContainText("one-sided row");
  await expect(openNote).toContainText("folder to descend");
  await expect(openNote).toContainText("d00");
  // Still at root: up-level remains absent, d00 still visible.
  await expect(page.getByTestId("folder-up-level")).toHaveCount(0);
  await expect(d00Row).toBeVisible();

  // 3. Two-sided directory (d01) activation via Enter: re-roots into subdir.
  // d01 is at index 56 in the mock engine fixture; page once to reveal it.
  const d01Row = page.locator('[data-testid^="folder-row-"][data-path="d01"]');
  if ((await d01Row.count()) === 0) {
    const more = page.getByTestId("folderview/load-more");
    if ((await more.count()) > 0) {
      await more.first().click();
    }
  }
  await expect(d01Row).toBeVisible();
  await d01Row.press("Enter");

  // Re-rooted: up-level button + breadcrumb visible, note cleared.
  await expect(page.getByTestId("folder-up-level")).toBeVisible();
  await expect(page.getByTestId("folder-breadcrumb")).toBeVisible();
  await expect(page.getByTestId("folder-breadcrumb")).toContainText("d01");
  await expect(page.getByTestId("folder-open-note")).toHaveCount(0);

  // Rows re-rooted: children of d01 render with relative paths (e.g. f00.txt).
  await expect(
    page.locator('[data-testid^="folder-row-"][data-path="f00.txt"]'),
  ).toBeVisible();
  // Parent / sibling rows (like d00) are not rendered inside d01.
  await expect(page.locator('[data-testid^="folder-row-"][data-path="d00"]')).toHaveCount(0);

  // 4. Chrome budget intact: firstRowTop <= 140px inside subdirectory.
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `firstRowTop budget intact (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  // 5. Up-level button returns to parent/root.
  await page.getByTestId("folder-up-level").click();
  await expect(page.getByTestId("folder-up-level")).toHaveCount(0);
  await expect(page.getByTestId("folder-breadcrumb")).toHaveCount(0);
  // Root rows restored.
  await expect(page.locator('[data-testid^="folder-row-"][data-path="d00"]')).toBeVisible();

  // 6. Two-sided directory activation via double-click also re-roots.
  if ((await page.locator('[data-testid^="folder-row-"][data-path="d01"]').count()) === 0) {
    const more = page.getByTestId("folderview/load-more");
    if ((await more.count()) > 0) {
      await more.first().click();
    }
  }
  const d01Row2 = page.locator('[data-testid^="folder-row-"][data-path="d01"]');
  await expect(d01Row2).toBeVisible();
  await d01Row2.dblclick();
  await expect(page.getByTestId("folder-up-level")).toBeVisible();
  await expect(
    page.locator('[data-testid^="folder-row-"][data-path="f00.txt"]'),
  ).toBeVisible();

  // 7. Breadcrumb root button also returns to root.
  await page.getByTestId("folder-breadcrumb-root").click();
  await expect(page.getByTestId("folder-up-level")).toHaveCount(0);
  await expect(page.locator('[data-testid^="folder-row-"][data-path="d00"]')).toBeVisible();

  // 8. Light + dark evidence.
  mkdirSync(EVID_DIR, { recursive: true });
  await page.screenshot({ path: `${EVID_DIR}/folder-descend-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-descend-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
