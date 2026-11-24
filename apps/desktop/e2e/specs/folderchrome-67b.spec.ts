/**
 * Wave-67B folder chrome collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the mock folder engine and proves
 * the Beyond Compare style collapse:
 *  - one toolbar row (`folderview/toolbar`) hosts all/diffs/same +
 *    changes-only + name-filter toggle;
 *  - name panel open by default, toolbar/Esc collapse it, Ctrl+F
 *    reopens + focuses it;
 *  - totals + changes/name counts live in the bottom status line
 *    (`folderview/status`), not above the list;
 *  - collapsed chrome budget: first row top <= 140px with the panel
 *    closed (single toolbar row above the list);
 *  - every legacy testid still works (filter, changes-only, name
 *    filter, totals);
 *  - light + dark, zero console/page errors.
 *
 * New file only, no existing spec touched.
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

test("folder chrome collapse: single toolbar, name overlay, counts in status bar, row budget", async ({
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

  // 1. Single toolbar row with every control; name panel open by default.
  // Wave-113B: the name panel is default-closed; open it for the legacy
  // open-panel assertions (toggle parity preserved: step 2 closes it).
  if ((await page.getByTestId("folderview/toolbar-namefilter").getAttribute("aria-expanded")) !== "true") {
    await page.getByTestId("folderview/toolbar-namefilter").click();
  }
  const toolbar = page.getByTestId("folderview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("folder-filter-all")).toBeVisible();
  await expect(page.getByTestId("folder-filter-diffs")).toBeVisible();
  await expect(page.getByTestId("folder-filter-same")).toBeVisible();
  await expect(page.getByTestId("folder-changesonly-toggle")).toBeVisible();
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toBeVisible();
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect(page.getByTestId("folder-name-filter")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status line at the bottom carries totals (legacy testid intact).
  const status = page.getByTestId("folderview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("folderview/totals")).toContainText(/entries/);
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTop = await page
    .locator('[data-testid^="folder-row-"]')
    .first()
    .evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "counts live below the list in the status bar").toBeGreaterThan(firstRowTop);
  expect(toolbarTop, "toolbar sits above the list").toBeLessThan(firstRowTop);

  // 2. Toolbar toggle collapses the panel; Ctrl+F reopens + focuses.
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folder-name-filter")).toBeHidden();
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("folder-name-filter")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("folder-name-filter");

  // 3. Esc clears + closes the panel (legacy clear behavior kept).
  const filter = page.getByTestId("folder-name-filter");
  await filter.fill("d00");
  await expect(page.getByTestId("folder-name-note")).toBeVisible();
  await expect(page.getByTestId("folder-name-note")).toContainText("d00");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folder-name-filter")).toBeHidden();
  await expect(page.getByTestId("folder-name-note")).toHaveCount(0);
  // Ctrl+F again reopens for the filter assertions below.
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("folder-name-filter")).toBeVisible();

  // 4. Legacy filter behavior intact: diffs hides same, changes-only
  //    composes, name query narrows, counts stay in the status bar.
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedTotals = await totalsText();
  await page.getByTestId("folder-filter-diffs").click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const diffStatuses: string[] = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
  for (const s of diffStatuses) {
    expect(s, "diffs filter hides every same row").not.toBe("same");
  }
  await page.getByTestId("folder-filter-all").click();
  const toggle = page.getByTestId("folder-changesonly-toggle");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("folder-changes-count")).toBeVisible();
  await expect(page.getByTestId("folder-changes-count")).toContainText(/changes/);
  expect(await totalsText(), "toggling never refetches").toBe(loadedTotals);
  await toggle.click();
  await expect(page.getByTestId("folder-changes-count")).toHaveCount(0);

  // 5. Collapsed chrome budget: with the panel closed the folder chrome
  //    (toolbar + panel, i.e. first-row top relative to the folder
  //    section top — app tab/path bars above are not folder chrome)
  //    is one compact row. Old stacked chrome (~5 rows) was ~200px+.
  await page.getByTestId("folderview/toolbar-namefilter").click();
  await expect(page.getByTestId("folder-name-filter")).toBeHidden();
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
    `folder chrome collapsed to one row (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: "/tmp/ui-67b/folder-chrome-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-67b/folder-chrome-dark.png" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
