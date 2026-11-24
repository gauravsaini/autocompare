/**
 * Wave-38 table export-CSV proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`; header is
 * `id | name | price`, 550 rows, 100 loaded) and proves:
 *  - the `Export CSV` button (`table-export`, native control, keyboard
 *    reachable — Enter on the focused button triggers it) serializes the
 *    currently-visible grid to a `table-export.csv` Blob download and
 *    shows an inline receipt (`table-export-receipt`, `role="status"`)
 *    with row/col counts (`100 rows × 3 cols`, `all 100 loaded rows`,
 *    `positional order`); the downloaded bytes start with
 *    `id,name,price` and hold header + 100 rows;
 *  - hiding `price` excludes it from the download (header `id,name`,
 *    101 lines, no price values) while the receipt stays honest
 *    (`100 rows × 2 cols`, `1 hidden column excluded`);
 *  - a find with hits narrows the export to hit rows only: `10.` lists
 *    `Hit 1 of 6`, export downloads header + 3 rows and the receipt
 *    notes the scope (`3 rows × 3 cols`, `6 hits to 3 rows`);
 *  - hiding every column refuses inline (receipt `all columns hidden`,
 *    no download fires, `window.__TABLE_EXPORT_LAST__` unchanged, never
 *    a crash), and show-all + re-export restores the full report;
 *  - light + dark PNGs land in `/tmp/ui-38/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock-IPC honesty: export is pure client-side serialization of loaded
 * rows — no new IPC commands are issued. Every assertion reads the DOM
 * or the downloaded bytes, never the network. Skip policy matches the
 * other specs (infra failures skip). New file only; does not touch any
 * existing spec.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";

const EVID_DIR = "/tmp/ui-38";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createTableSession(page: Page, left: string, right: string): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function readDownload(page: Page, testId: string): Promise<string> {
  const [download] = await Promise.all([
    page.waitForEvent("download", { timeout: 10000 }),
    page.getByTestId(testId).click(),
  ]);
  expect(download.suggestedFilename()).toBe("table-export.csv");
  const path = await download.path();
  expect(path).toBeTruthy();
  return fs.readFileSync(path as string, "utf-8");
}

test("table export: visible-grid CSV download + receipt, hidden excluded, find scope honest, all-hidden refuses, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page, "/tmp/wave38-left.csv", "/tmp/wave38-right.csv");
  const view = page.getByTestId("view-table");
  const exportBtn = page.getByTestId("table-export");
  const receipt = page.getByTestId("table-export-receipt");

  // Button renders, no receipt before the first export, grid fully loaded.
  await expect(exportBtn).toBeVisible();
  await expect(exportBtn).toHaveText("Export CSV");
  await expect(receipt).toHaveCount(0);
  await expect(page.getByTestId("table-cols-count")).toContainText("100 / 550 rows");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Keyboard: Enter on the focused Export button downloads the full
  // visible grid (header + 100 loaded rows, positional order).
  await exportBtn.focus();
  const [fullDownload] = await Promise.all([
    page.waitForEvent("download", { timeout: 10000 }),
    page.keyboard.press("Enter"),
  ]);
  expect(fullDownload.suggestedFilename()).toBe("table-export.csv");
  const fullPath = await fullDownload.path();
  const fullText = fs.readFileSync(fullPath as string, "utf-8");
  const fullLines = fullText.trim().split("\n");
  expect(fullLines[0].trim()).toBe("id,name,price");
  expect(fullLines).toHaveLength(101);
  expect(fullLines[1].trim()).toBe("R0001,item-0001,10.25");
  await expect(receipt).toContainText("100 rows × 3 cols");
  await expect(receipt).toContainText("all 100 loaded rows");
  await expect(receipt).toContainText("positional order");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Hide `price`: export excludes the hidden column from both header and
  // rows, receipt counts + exclusion stay honest.
  await page.getByTestId("table-cols-toggle").click();
  await page.getByTestId("table-col-visibility-2").uncheck();
  await expect(page.getByTestId("table-cols-count")).toContainText("2 of 3 cols (1 hidden)");
  await expect(receipt).toHaveCount(0);
  const hiddenText = await readDownload(page, "table-export");
  const hiddenLines = hiddenText.trim().split("\n");
  expect(hiddenLines[0].trim()).toBe("id,name");
  expect(hiddenLines).toHaveLength(101);
  expect(hiddenLines[1].trim()).toBe("R0001,item-0001");
  expect(hiddenText).not.toContain("10.25");
  await expect(receipt).toContainText("100 rows × 2 cols");
  await expect(receipt).toContainText("1 hidden column excluded");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Find with hits narrows the export to hit rows and the receipt names
  // the scope: `10.` lives in `price` only, so restore all columns first.
  await page.getByTestId("table-cols-show-all").click();
  await expect(page.getByTestId("table-cols-count")).toContainText("3 cols");
  // Show-all leaves the menu open; close it via the toggle so later
  // toggle clicks deterministically re-open it.
  await page.getByTestId("table-cols-toggle").click();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  // Wave-113A preamble (zero assertion change): find panel is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();
  await page.getByTestId("table-find-input").fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 1 of 6", {
    timeout: 10000,
  });
  const findText = await readDownload(page, "table-export");
  const findLines = findText.trim().split("\n");
  expect(findLines[0].trim()).toBe("id,name,price");
  expect(findLines).toHaveLength(4);
  await expect(receipt).toContainText("3 rows × 3 cols");
  await expect(receipt).toContainText('find "10.": 6 hits to 3 rows');
  await expect(page.getByRole("alert")).toHaveCount(0);

  // All columns hidden refuses inline: receipt note, no download, hook
  // untouched, never a crash.
  await page.getByTestId("table-cols-toggle").click();
  await page.getByTestId("table-col-visibility-0").uncheck();
  await page.getByTestId("table-col-visibility-1").uncheck();
  await page.getByTestId("table-col-visibility-2").uncheck();
  await expect(page.getByTestId("table-cols-count")).toContainText("0 of 3 cols (3 hidden)");
  await expect(receipt).toHaveCount(0);
  const before = await page.evaluate(
    () => (window as unknown as Record<string, unknown>)["__TABLE_EXPORT_LAST__"],
  );
  await exportBtn.click();
  await expect(receipt).toContainText("all columns hidden");
  const after = await page.evaluate(
    () => (window as unknown as Record<string, unknown>)["__TABLE_EXPORT_LAST__"],
  );
  expect(after).toEqual(before);
  const noDownload = await page
    .waitForEvent("download", { timeout: 1500 })
    .then(
      () => false,
      () => true,
    );
  expect(noDownload).toBe(true);
  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Restore + re-export so the evidence shots capture a full grid with a
  // fresh receipt in both themes.
  await page.getByTestId("table-cols-show-all").click();
  await page.getByTestId("table-find-input").fill("");
  await page.getByTestId("table-find-search").click();
  await readDownload(page, "table-export");
  await expect(receipt).toContainText("100 rows × 3 cols");
  await page.screenshot({ path: `${EVID_DIR}/table-export-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-export-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
