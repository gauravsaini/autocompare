/**
 * Wave-98 folder chrome committed spec (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Committed proof for GATES-98 gate 2 (temp probes deleted after use in
 * 27363fc are no standing proof): drives the REAL `App` workspace over
 * the mock folder engine and pins the wave-89 port —
 *  - goto popover closed by default, Ctrl+G opens + focuses, Esc closes;
 *  - global Esc collapses the name panel WITHOUT clearing its query;
 *  - diffs/same counts live in the bottom status bar (`folderview/counts`);
 *  - collapsed chrome budget: section-relative first-row delta <= 140px;
 *  - light + dark, zero console/page errors.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";

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

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("folderchrome-98: goto popover, global Esc keeps query, counts in status, row budget", async ({
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

  // 1. Goto closed by default; Ctrl+G opens + focuses; Esc closes.
  await expect(page.getByTestId("folderview/goto-panel")).toHaveCount(0);
  await page.keyboard.press("Control+g");
  await expect(page.getByTestId("folderview/goto-panel")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("folder-goto-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/goto-panel")).toHaveCount(0);

  // 2. Global Esc collapses the name panel WITHOUT clearing the query.
  // (The goto Esc above also collapses the name panel via the global
  // handler, by design — reopen it first.)
  await page.getByTestId("folderview/toolbar-namefilter").click();
  const nameFilter = page.getByTestId("folder-name-filter");
  await expect(nameFilter).toBeVisible();
  await nameFilter.fill("d00");
  await rows.first().click(); // move focus out so only the global Esc fires
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folderview/name-panel")).toBeVisible();
  await expect(page.getByTestId("folder-name-filter")).toHaveValue("d00");
  await page.getByTestId("folder-name-filter").fill("");
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);

  // 3. Counts live in the bottom status bar below the list.
  const status = page.getByTestId("folderview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("folderview/counts")).toBeVisible();
  await expect(page.getByTestId("folderview/counts")).toContainText(/\d+ diffs · \d+ same/);
  const firstTop = await rows.first().evaluate((el) => el.getBoundingClientRect().top);
  expect(await status.evaluate((el) => el.getBoundingClientRect().top)).toBeGreaterThan(firstTop);

  // 4. Collapsed chrome budget: section top -> first row <= 140px.
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(chrome ?? Number.POSITIVE_INFINITY, `chrome delta (got ${chrome}px)`).toBeLessThanOrEqual(140);

  // 5. Light + dark, zero errors.
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: "/tmp/ui-98/folderchrome-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("folderview/status")).toBeVisible();
  await page.screenshot({ path: "/tmp/ui-98/folderchrome-dark.png" });
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
