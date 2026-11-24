/**
 * Wave-41 folder keyboard-navigation proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine and proves
 * Beyond Compare style arrow-key navigation, fully client-side:
 *  - ArrowDown moves selection to the next FILE row (detail follows,
 *    DOM focus follows), ArrowUp moves back;
 *  - End jumps to the last file, Home to the first; both clamp at the
 *    ends (no wrap, no crash, selection stays);
 *  - directory rows are skipped for selection;
 *  - navigation never refetches (totals label unchanged);
 *  - light + dark PNGs land in `/tmp/ui-41/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-41";

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

/** data-paths of visible FILE rows (no collapse toggle inside), in order. */
async function filePaths(page: Page): Promise<string[]> {
  return page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes
      .filter((n) => n.querySelector('[data-testid^="folder-collapse-"]') === null)
      .map((n) => n.getAttribute("data-path") ?? ""),
  );
}

function rowByPath(page: Page, path: string) {
  return page.locator(`[data-testid^="folder-row-"][data-path="${path}"]`);
}

async function focusedPath(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-path") ?? "") : "";
  });
}

test("folder keyboard nav: arrows move file selection, home/end jump, ends clamp", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createFolderSession(page);
  const view = page.getByTestId("view-folder");
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedTotals = await totalsText();

  const paths = await filePaths(page);
  expect(paths.length, "needs at least two file rows to prove movement").toBeGreaterThan(1);
  const [first, second] = paths;
  const last = paths[paths.length - 1];

  // Click selects the first file; click also focuses its row.
  await rowByPath(page, first).click();
  await expect(rowByPath(page, first)).toHaveAttribute("data-selected", "true");

  // ArrowDown: selection + detail + DOM focus all move to the next file.
  await rowByPath(page, first).press("ArrowDown");
  await expect(rowByPath(page, second)).toHaveAttribute("data-selected", "true");
  await expect(rowByPath(page, first)).toHaveAttribute("data-selected", "false");
  await expect(
    rowByPath(page, second).locator('[data-testid^="folder-detail-"]'),
  ).toBeVisible();
  expect(await focusedPath(page), "focus follows selection").toBe(second);

  // ArrowUp: back to the first file.
  await rowByPath(page, second).press("ArrowUp");
  await expect(rowByPath(page, first)).toHaveAttribute("data-selected", "true");
  await expect(rowByPath(page, second)).toHaveAttribute("data-selected", "false");
  expect(await focusedPath(page), "focus follows selection").toBe(first);

  // ArrowUp at the first file clamps (no wrap, no crash).
  await rowByPath(page, first).press("ArrowUp");
  await expect(rowByPath(page, first)).toHaveAttribute("data-selected", "true");
  expect(await focusedPath(page)).toBe(first);

  // End jumps to the last file; ArrowDown there clamps too.
  await rowByPath(page, first).press("End");
  await expect(rowByPath(page, last)).toHaveAttribute("data-selected", "true");
  await expect(
    rowByPath(page, last).locator('[data-testid^="folder-detail-"]'),
  ).toBeVisible();
  expect(await focusedPath(page)).toBe(last);
  await rowByPath(page, last).press("ArrowDown");
  await expect(rowByPath(page, last)).toHaveAttribute("data-selected", "true");

  // Home returns to the first file.
  await rowByPath(page, last).press("Home");
  await expect(rowByPath(page, first)).toHaveAttribute("data-selected", "true");
  expect(await focusedPath(page)).toBe(first);

  expect(await totalsText(), "navigation never refetches").toBe(loadedTotals);
  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folder-nav-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-nav-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
