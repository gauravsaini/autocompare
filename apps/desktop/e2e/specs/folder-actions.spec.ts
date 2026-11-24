/**
 * Wave-28 folder row-actions proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`) and proves
 * Beyond Compare style row actions:
 *  - clicking a file row selects it (`data-selected="true"`) and shows
 *    an inline detail (`folder-detail-<i>`) rendered from the
 *    already-loaded entry (relPath, status, left/right size+modified);
 *    no new IPC is issued (totals label unchanged);
 *  - the per-row `folder-copy-<i>` button writes via
 *    `navigator.clipboard` with an inline `copied` receipt
 *    (`folder-copied-<i>`); where the headless browser has no
 *    clipboard, an honest inline note (`folder-copy-note`) renders
 *    instead — either branch passes, a crash never happens;
 *  - keyboard Enter on a focused file row activates it (selects +
 *    shows its detail);
 *  - folder rows still collapse/expand descendants;
 *  - the all/diffs/same filter still works client-side;
 *  - light + dark PNGs land in `/tmp/ui-28/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-28";

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

test("folder row actions: click detail + copy-path receipt + enter + toggle + filter", async ({
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

  // Load a second window so file rows with left+right data exist.
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedBefore = await totalsText();
  const more = page.getByTestId("folderview/load-more");
  if ((await more.count()) > 0) {
    await more.first().click();
    await expect.poll(totalsText, { timeout: 10000 }).not.toBe(loadedBefore);
  }
  const loadedTotals = await totalsText();

  // Pick a file row (no collapse toggle inside) via its data-path.
  const fileRow = page.locator(
    '[data-testid^="folder-row-"][data-path="d00/f00.txt"]',
  );
  await expect(fileRow).toHaveCount(1);
  await expect(fileRow.locator('[data-testid^="folder-collapse-"]')).toHaveCount(0);

  // Click selects the file row and shows inline detail from loaded data.
  await fileRow.click();
  await expect(fileRow).toHaveAttribute("data-selected", "true");
  const detail = fileRow.locator('[data-testid^="folder-detail-"]');
  await expect(detail).toBeVisible();
  await expect(detail).toContainText("d00/f00.txt");
  await expect(detail).toContainText("left:");
  await expect(detail).toContainText("right:");
  expect(await totalsText(), "selecting never refetches").toBe(loadedTotals);

  // Copy-path: clipboard receipt OR honest fallback note (headless
  // webkit often has no clipboard) — either way inline, never a crash.
  const copyBtn = fileRow.locator('[data-testid^="folder-copy-"]');
  await expect(copyBtn).toBeVisible();
  await copyBtn.click();
  const copiedReceipt = fileRow.locator('[data-testid^="folder-copied-"]');
  const fallbackNote = page.getByTestId("folder-copy-note");
  await expect
    .poll(
      async () =>
        ((await copiedReceipt.count()) > 0 ? "receipt" : "") +
        ((await fallbackNote.count()) > 0 ? "note" : ""),
      { timeout: 5000 },
    )
    .not.toBe("");
  if ((await copiedReceipt.count()) > 0) {
    await expect(copiedReceipt).toContainText("copied");
  } else {
    await expect(fallbackNote).toContainText(/clipboard unavailable|copy failed/);
  }
  // Row stays selected after copy (copy button stops propagation).
  await expect(fileRow).toHaveAttribute("data-selected", "true");

  // Keyboard Enter activates the focused row: focus a different file row.
  const otherFile = page.locator(
    '[data-testid^="folder-row-"][data-path="d00/f01.txt"]',
  );
  if ((await otherFile.count()) === 1) {
    await otherFile.focus();
    await page.keyboard.press("Enter");
    await expect(otherFile).toHaveAttribute("data-selected", "true");
    await expect(otherFile.locator('[data-testid^="folder-detail-"]')).toBeVisible();
    await expect(fileRow).toHaveAttribute("data-selected", "false");
  } else {
    // Fallback: Enter on the same row keeps it selected.
    await fileRow.focus();
    await page.keyboard.press("Enter");
    await expect(fileRow).toHaveAttribute("data-selected", "true");
  }

  // Folder rows still collapse/expand descendants.
  const collapse0 = page.getByTestId("folder-collapse-0");
  await expect(collapse0).toBeVisible();
  const descendant = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  const beforeCollapse = await rows.count();
  await collapse0.click();
  await expect(descendant).toHaveCount(0);
  expect(await rows.count(), "collapsing d00 hides its descendants").toBeLessThan(
    beforeCollapse,
  );
  await collapse0.click();
  await expect(descendant).toHaveCount(1);

  // Filter interaction unchanged: diffs hides same rows, all restores.
  const allBtn = page.getByTestId("folder-filter-all");
  const diffsBtn = page.getByTestId("folder-filter-diffs");
  const sameBtn = page.getByTestId("folder-filter-same");
  await diffsBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const diffStatuses: string[] = await page.$$eval(
    '[data-testid^="folder-row-"]',
    (nodes) => nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
  for (const s of diffStatuses) {
    expect(s, "diffs filter hides every same row").not.toBe("same");
  }
  expect(await totalsText(), "filtering never refetches").toBe(loadedTotals);
  await sameBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  await allBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folder-actions-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-actions-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
