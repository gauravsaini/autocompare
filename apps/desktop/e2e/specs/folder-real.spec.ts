/**
 * Wave-9 folder-real proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * The mock IPC branch (`src/ipc/client.ts`) serves the committed
 * real-engine `dumped.json` (`e2e/helpers/dumped.json`, produced by
 * `src-tauri/examples/dump_fixtures.rs`) with real paging semantics.
 * This spec drives the REAL `App` workspace (SessionManager dialog ->
 * tab -> ViewRouter -> FolderView) over the wave-4A fixture dirs and
 * proves end-to-end that FolderView renders REAL engine data:
 *  - loaded/total counts equal the engine node count (590),
 *  - one-sided statuses (`leftOnly` + `rightOnly`) are present,
 *  - past the 500-node threshold the virtualized scroll container
 *    (not the plain list) is mounted,
 *  - rendered nodes never overlap (boundingBox check),
 *  - light + dark PNGs land in `/tmp/ui-9/` (never in-repo).
 *
 * Stable-testids-only rule: every locator below was inventoried from
 * `apps/desktop/src` before writing; node testids
 * (`folderview/node-<i>`) plus the wave-9 `folderview/totals`,
 * `folderview/scroller`, and `folderview/load-more` hooks already
 * exist, so no testid was added for this spec. Unlike the wave-4A
 * sweep, NOTHING skips when hooks are missing: missing hooks fail,
 * because wave-9 owns the hooks. Skip policy for infra (dev server
 * unreachable, browsers missing offline) matches the other specs.
 */
import { expect, test, type Page } from "@playwright/test";
import { COUNTS, FOLDER_NODES, FOLDER_STATUSES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave4a-left-dir";
const RIGHT_DIR = "/tmp/wave4a-right-dir";
const EVID_DIR = "/tmp/ui-9";

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

/**
 * Clicks "Load more" until the view reports its full total (bounded).
 * Polls on the button label (`Load more (loaded/total)`), which changes
 * after every appended window or vanishes at the end.
 */
async function expandFully(page: Page): Promise<void> {
  for (let i = 0; i < 14; i += 1) {
    const more = page.getByTestId("folderview/load-more");
    if ((await more.count()) === 0) return;
    const before = ((await more.first().textContent()) ?? "").trim();
    await more.first().click();
    try {
      await expect
        .poll(
          async () => {
            const btn = page.getByTestId("folderview/load-more");
            if ((await btn.count()) === 0) return "done";
            return ((await btn.first().textContent()) ?? "").trim();
          },
          { timeout: 5000 },
        )
        .not.toBe(before);
    } catch {
      return;
    }
  }
}

async function readTotals(page: Page): Promise<{ loaded: number; total: number }> {
  const text = ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const match = /(\d+)\s*\/\s*(\d+)\s*entries/.exec(text);
  expect(match, `totals label parses as "loaded / total entries" (got: ${text})`).not.toBeNull();
  return { loaded: Number(match?.[1]), total: Number(match?.[2]) };
}

test("folder real-data proof: engine counts, one-sided statuses, no overlap, light+dark", async ({
  page,
}) => {
  // Ground truth comes from the dumped engine output, not hand-written mocks.
  // The fixture must stay past the virtualization threshold or the
  // virtualization proof below is vacuous.
  expect(
    FOLDER_NODES.length,
    "engine folder fixture stays past the 500-node virtualization threshold",
  ).toBeGreaterThan(500);
  expect(FOLDER_STATUSES["leftOnly"] ?? 0, "engine has left-only nodes").toBeGreaterThan(0);
  expect(FOLDER_STATUSES["rightOnly"] ?? 0, "engine has right-only nodes").toBeGreaterThan(0);

  await createFolderSession(page);
  const view = page.getByTestId("view-folder");

  // Real folder_compare IPC path: node hooks render (no skip on missing).
  const nodes = page.locator('[data-testid^="folderview/node-"]');
  await expect.poll(async () => nodes.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // Real engine shape: first node follows engine relPath sort order.
  await expect(nodes.first()).toContainText(FOLDER_NODES[0]?.relPath ?? "d00");

  // Totals match the engine node count from the first window on.
  const first = await readTotals(page);
  expect(first.total, "view total = engine node count").toBe(COUNTS.folderNodes);
  expect(first.total, "view total = dumped folderNodes length").toBe(FOLDER_NODES.length);
  expect(first.loaded, "first window pages 50 entries").toBeGreaterThanOrEqual(50);

  // One-sided statuses keep their verdict classes on real data.
  await expect(view.locator(".fv-status-leftOnly").first()).toBeVisible();
  await expect(view.locator(".fv-status-rightOnly").first()).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Page through every window, then prove loaded count = engine count.
  await expandFully(page);
  const full = await readTotals(page);
  expect(full.loaded, "loaded node count = engine node count").toBe(COUNTS.folderNodes);
  expect(full.total, "total node count = engine node count").toBe(COUNTS.folderNodes);
  await expect(view).toContainText(`${COUNTS.folderNodes} / ${COUNTS.folderNodes} entries`);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Past the 500-node threshold the virtualized scroll container
  // (not the plain list) is mounted.
  await expect(page.getByTestId("folderview/scroller")).toBeVisible();
  await expect(page.getByTestId("folderview/list")).toHaveCount(0);

  // Rendered folder nodes never overlap (boundingBox proof).
  await assertNoOverlap(page, '[data-testid^="folderview/node-"]');

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folder-real-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-real-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);
});
