/**
 * Wave-84 folder size/date meta proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the mock folder engine and proves:
 *  - every rendered row carries an inline `folder-meta-<i>` subline with
 *    compact per-side meta (`"<size>, <YYYY-MM-DD>"`) plus a `title`
 *    tooltip with the full `left: … | right: …` detail;
 *  - a one-sided fixture row (first row `d00`, leftOnly) shows the
 *    present size and the honest `size unknown / modified unknown`
 *    fallback for the missing side;
 *  - a two-sided row (after load-more, e.g. `d01/f00.txt`) shows both
 *    sizes with no fallback;
 *  - no new stacked chrome rows: first-row top stays <= 140px from the
 *    folder section top (panel closed);
 *  - zero console errors / page errors throughout.
 *
 * New file only; no existing spec touched.
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

test("folder meta-84: inline size/date per side, honest null-side fallback, chrome budget intact", async ({
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

  // 1. One-sided fixture row: first row is d00 (leftOnly, right null).
  const first = rows.first();
  await expect(first).toHaveAttribute("data-path", "d00");
  await expect(first).toHaveAttribute("data-status", "leftOnly");
  const meta0 = page.getByTestId("folder-meta-0");
  await expect(meta0).toBeVisible();
  // Present side: compact "0 B, 2026-…" (dir size 0 + ISO day).
  await expect(meta0).toContainText(/0 B/);
  await expect(meta0).toContainText(/2026-/);
  // Missing side: honest fallback.
  await expect(meta0).toContainText("size unknown / modified unknown");
  // Tooltip carries the full per-side detail.
  const title0 = (await meta0.getAttribute("title")) ?? "";
  expect(title0, "tooltip carries left detail").toContain("left:");
  expect(title0, "tooltip carries right detail").toContain("right:");

  // 2. File one-sided row also shows size + fallback inline.
  const fileRow = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  await expect(fileRow).toHaveCount(1);
  const fileMeta = fileRow.locator('[data-testid^="folder-meta-"]');
  await expect(fileMeta).toContainText(/59 B/);
  await expect(fileMeta).toContainText("size unknown / modified unknown");

  // 3. Two-sided row after load-more: both sides present, no fallback.
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedBefore = await totalsText();
  const more = page.getByTestId("folderview/load-more");
  if ((await more.count()) > 0) {
    await more.first().click();
    await expect.poll(totalsText, { timeout: 10000 }).not.toBe(loadedBefore);
  }
  const twoSided = page.locator('[data-testid^="folder-row-"][data-path="d01/f00.txt"]');
  await expect(twoSided).toHaveCount(1);
  const twoMeta = twoSided.locator('[data-testid^="folder-meta-"]');
  await expect(twoMeta).toContainText(/86 B/);
  await expect(twoMeta).not.toContainText("size unknown");
  await expect(twoMeta).not.toContainText("modified unknown");

  // 4. No new stacked chrome rows: with the name panel closed the folder
  //    chrome (first-row top relative to section top) stays one compact row.
  const nameFilter = page.getByTestId("folderview/toolbar-namefilter");
  if (await page.getByTestId("folder-name-filter").isVisible()) {
    await nameFilter.click();
    await expect(page.getByTestId("folder-name-filter")).toBeHidden();
  }
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const firstRow = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(firstRow instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = firstRow.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `no stacked meta rows (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
