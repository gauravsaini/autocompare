/**
 * Wave-16 center hunk copy proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 15 hunks) and proves
 * the Beyond Compare style center gutter:
 *  - every hunk in the loaded range mounts one control pair between the
 *    panes (`hunk-copy-left-N` take-left copies left to right,
 *    `hunk-copy-right-N` take-right mirrors, N = hunk ordinal over
 *    consecutive non-equal rows, the same grouping the ruler uses);
 *  - clicking take-left on hunk 0 (a left-only run) fills the right pane
 *    cells with the left text and clears the hunk (rows flip to equal,
 *    the pair count drops by one);
 *  - take-right on the next hunk (a right-only run) mirrors this
 *    (left cells fill with the right text);
 *  - light + dark PNGs land in `/tmp/ui-16/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: the mock IPC adapter has no `text_copy_section` fixture
 * (unknown commands resolve `[]`), and the real backend keeps stored
 * rows read-only for copies, so the view re-reads the loaded range and
 * then applies the copy to its locally held fixture rows; every
 * assertion below reads the DOM, never the network. The spec stays at
 * 400 loaded rows (two pages) so the pane renders non-virtualized and
 * every loaded hunk mounts its pair without scrolling. Skip policy
 * matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-16";

/** Hunk-start row indices from the rendered view rows (consecutive non-equal runs). */
async function viewHunkStarts(page: Page): Promise<number[]> {
  return page.$$eval('[data-testid^="textview/row-"]', (nodes) => {
    const rows = nodes
      .map((n) => ({
        index: Number((n.getAttribute("data-testid") ?? "").replace("textview/row-", "")),
        kind: n.getAttribute("data-kind") ?? "",
      }))
      .sort((a, b) => a.index - b.index);
    const starts: number[] = [];
    let inHunk = false;
    for (const row of rows) {
      const hunkRow = row.kind !== "equal";
      if (hunkRow && !inHunk) starts.push(row.index);
      inHunk = hunkRow;
    }
    return starts;
  });
}

async function pairCount(page: Page): Promise<number> {
  return page.locator('[data-testid^="hunk-copy-left-"]').count();
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("center gutter copies hunks left/right on the demo text diff", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // Second page: 400 loaded rows stay non-virtualized (<= 500), so every
  // loaded hunk mounts its gutter pair without scrolling.
  await page.getByRole("button", { name: /Load more/ }).click();
  await expect(page.getByTestId("textview/status")).toContainText("400 / 644 rows");

  // Loaded hunks: single-row and multi-row runs starting at 6, 20, 51,
  // 98, 145, 192, 239, 286, 333, 380 (all fully inside rows 0..399).
  const starts = await viewHunkStarts(page);
  expect(starts).toEqual([6, 20, 51, 98, 145, 192, 239, 286, 333, 380]);

  // One control pair per hunk, sitting between the panes (after the left
  // cell, before the right cell) on the hunk's first row.
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(starts.length);
  for (let n = 0; n < starts.length; n += 1) {
    await expect(page.getByTestId(`hunk-copy-left-${n}`)).toBeVisible();
    await expect(page.getByTestId(`hunk-copy-right-${n}`)).toBeVisible();
  }
  const pairOrder = await page.$$eval('[data-testid^="hunk-copy-left-"]', (nodes) =>
    nodes.map((n) => {
      const row = n.closest('[data-testid^="textview/row-"]');
      const cells = row?.children[0]?.children ?? [];
      return Array.from(cells).indexOf(n.closest("span") as Element);
    }),
  );
  for (const cellIndex of pairOrder) expect(cellIndex).toBe(2);

  // Take-left on hunk 0 (left-only run rows 6..9): the right pane fills
  // with the left text and the hunk clears to equal.
  const rightOf = (row: number): ReturnType<Page["locator"]> =>
    page.getByTestId(`textview/row-${row}`).locator('div[title="right"]');
  await expect(rightOf(6)).toHaveText("");
  await page.getByTestId("hunk-copy-left-0").click();
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", "equal");
  for (const [row, text] of [
    [6, "removed line A"],
    [7, "removed line B"],
    [8, "removed line C"],
    [9, "removed line D"],
  ] as const) {
    await expect(page.getByTestId(`textview/row-${row}`)).toHaveAttribute("data-kind", "equal");
    await expect(rightOf(row)).toHaveText(text);
  }
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(starts.length - 1);
  expect(await viewHunkStarts(page)).toEqual([20, 51, 98, 145, 192, 239, 286, 333, 380]);

  // Take-right mirrors on the next hunk (right-only run rows 20..23):
  // the left pane fills with the right text and that hunk clears too.
  const leftOf = (row: number): ReturnType<Page["locator"]> =>
    page.getByTestId(`textview/row-${row}`).locator('div[title="left"]');
  await expect(leftOf(20)).toHaveText("");
  await page.getByTestId("hunk-copy-right-0").click();
  await expect(page.getByTestId("textview/row-20")).toHaveAttribute("data-kind", "equal");
  for (const [row, text] of [
    [20, "inserted extra line I"],
    [21, "inserted extra line II"],
    [22, "inserted extra line III"],
    [23, "inserted extra line IV"],
  ] as const) {
    await expect(page.getByTestId(`textview/row-${row}`)).toHaveAttribute("data-kind", "equal");
    await expect(leftOf(row)).toHaveText(text);
  }
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(starts.length - 2);

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hunk-copy-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hunk-copy-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
