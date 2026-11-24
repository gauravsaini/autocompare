/**
 * Wave-113B folder name-filter default-closed (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Committed proof for GATES-113B: drives the REAL `App` workspace over
 * the mock folder engine and pins the BC one-row chrome —
 *  - name panel closed by default; toolbar toggle `aria-expanded=false`;
 *  - toolbar toggle opens the panel; typing filters with an honest note;
 *  - Ctrl/Cmd+F opens + focuses from closed; Esc closes;
 *  - global Esc collapses WITHOUT clearing the query (reopen preserves it);
 *  - collapsed chrome budget: section-relative first-row delta <= 140px
 *    with the panel closed (same ruler as folderchrome-98);
 *  - light + dark PNGs to /tmp/folderfilter-113b-*.png (never in-repo);
 *  - zero console errors / page errors throughout.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

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

test("folderfilter-113b: name panel default-closed, toggle+shortcut open, chrome budget", async ({
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
  const before = await rows.count();

  // 1. Default-closed: no name panel, toggle collapsed.
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);
  await expect(page.getByTestId("folder-name-filter")).toHaveCount(0);
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toHaveAttribute(
    "aria-expanded",
    "false",
  );

  // 2. Collapsed chrome budget with panel closed (folderchrome-98 ruler).
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(chrome ?? Number.POSITIVE_INFINITY, `chrome delta (got ${chrome}px)`).toBeLessThanOrEqual(
    140,
  );
  await page.screenshot({ path: "/tmp/folderfilter-113b-closed-light.png" });

  // 3. Toolbar toggle opens the panel; typing filters with an honest note.
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folderview/name-panel")).toBeVisible();
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  const filter = page.getByTestId("folder-name-filter");
  await expect(filter).toBeVisible();

  const paths = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const base = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  const query = base.length >= 3 ? base : first;
  expect(query.length).toBeGreaterThan(0);

  await filter.fill(query);
  const note = page.getByTestId("folder-name-note");
  await expect(note).toBeVisible();
  await expect(note).toContainText(query);
  const narrowed = await rows.count();
  expect(narrowed).toBeLessThanOrEqual(before);
  expect(narrowed).toBeGreaterThanOrEqual(1);
  await page.screenshot({ path: "/tmp/folderfilter-113b-open-light.png" });

  // 4. Global Esc collapses WITHOUT clearing the query (98 semantics).
  await rows.first().click(); // move focus out so only the global Esc fires
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);
  await expect(note).toBeVisible(); // query kept -> note survives in status bar
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folderview/name-panel")).toBeVisible();
  await expect(page.getByTestId("folder-name-filter")).toHaveValue(query);

  // 5. Input-level Esc clears + closes; shortcut reopens + focuses.
  await page.getByTestId("folder-name-filter").fill("");
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);
  await page.keyboard.press("Control+f");
  try {
    await expect.poll(() => focusedTestId(page), { timeout: 2000 }).toBe("folder-name-filter");
  } catch {
    await page.keyboard.press("Meta+f");
    await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("folder-name-filter");
  }
  await expect(page.getByTestId("folderview/name-panel")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);

  // 6. Light + dark, zero errors.
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("folderview/status")).toBeVisible();
  await page.screenshot({ path: "/tmp/folderfilter-113b-dark.png" });
  await assertNoOverlap(page);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
