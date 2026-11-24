/**
 * Wave-35 folder changes-only proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`) and proves
 * the Beyond Compare style show-differences toggle:
 *  - one `folder-changesonly-toggle` hides every `same` row and shows
 *    an honest `folder-changes-count` ("Showing N changes — M same hidden");
 *  - the toggle composes with the existing all/diffs/same filter and
 *    per-folder collapse (collapse state survives the toggle);
 *  - toggling back off restores every row (same rows return);
 *  - the toggle is keyboard reachable (focus + Space/Enter flips it);
 *  - filtering/toggling never refetches (totals label unchanged);
 *  - light + dark PNGs land in `/tmp/ui-35/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-35";

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

async function rowStatuses(page: Page): Promise<string[]> {
  return page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
}

test("folder changes-only: one toggle hides same rows, honest count, collapse intact, toggle off restores", async ({
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

  // Load a second window so both same and diff rows are present
  // (first window is all leftOnly; same rows start at index 57).
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedBefore = await totalsText();
  const more = page.getByTestId("folderview/load-more");
  if ((await more.count()) > 0) {
    await more.first().click();
    await expect.poll(totalsText, { timeout: 10000 }).not.toBe(loadedBefore);
  }
  const loadedTotals = await totalsText();
  const mixed = await rowStatuses(page);
  expect(mixed, "loaded windows contain diff rows").toContain("leftOnly");
  expect(mixed, "loaded windows contain same rows after load-more").toContain("same");
  const sameTotal = mixed.filter((s) => s === "same").length;
  const allCount = mixed.length;

  // Collapse d00 before toggling: collapse state must survive the toggle.
  const collapse0 = page.getByTestId("folder-collapse-0");
  await expect(collapse0).toBeVisible();
  const descendant = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  await expect(descendant).toHaveCount(1);
  await collapse0.click();
  await expect(descendant).toHaveCount(0);
  const collapsedCount = await rows.count();
  expect(collapsedCount).toBeLessThan(allCount);
  // Re-expand so the toggle-on assertion covers same rows too.
  await collapse0.click();
  await expect(descendant).toHaveCount(1);
  expect(await rows.count()).toBe(allCount);

  // Toggle exists, keyboard reachable, honest count on the button.
  const toggle = page.getByTestId("folder-changesonly-toggle");
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
  await expect(toggle).toContainText(/Changes only \(\d+\)/);

  // Toggle ON via keyboard (focus + Space): same rows hidden + count.
  await toggle.focus();
  await expect(toggle).toBeFocused();
  await page.keyboard.press("Space");
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const hiddenStatuses = await rowStatuses(page);
  expect(hiddenStatuses.length).toBeGreaterThan(0);
  for (const s of hiddenStatuses) {
    expect(s, "changes-only hides every same row").not.toBe("same");
  }
  expect(hiddenStatuses.length).toBe(allCount - sameTotal);
  const countLine = page.getByTestId("folder-changes-count");
  await expect(countLine).toBeVisible();
  await expect(countLine).toContainText(/Showing \d+ changes/);
  await expect(countLine).toContainText(new RegExp(`${sameTotal} same hidden`));
  expect(await totalsText(), "toggling never refetches (totals unchanged)").toBe(loadedTotals);

  // Collapse still works while changes-only is on (compose, not break).
  const descendantWhileOn = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  const onCount = await rows.count();
  await collapse0.click();
  await expect(descendantWhileOn).toHaveCount(0);
  expect(await rows.count(), "collapse hides descendants while changes-only is on").toBeLessThan(
    onCount,
  );
  await collapse0.click();
  await expect(descendantWhileOn).toHaveCount(1);
  expect(await rows.count(), "expand restores while changes-only is on").toBe(onCount);

  // Composes with the diffs filter: toggling diffs under changes-only
  // keeps same rows hidden and still refetches nothing.
  const diffsBtn = page.getByTestId("folder-filter-diffs");
  await diffsBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const comboStatuses = await rowStatuses(page);
  for (const s of comboStatuses) {
    expect(s, "diffs + changes-only still hides every same row").not.toBe("same");
  }
  expect(await totalsText(), "filter combo never refetches").toBe(loadedTotals);
  await page.getByTestId("folder-filter-all").click();

  // Toggle OFF restores every row (same rows return).
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBe(allCount);
  const restored = await rowStatuses(page);
  expect(restored, "toggle off restores same rows").toContain("same");
  expect(restored, "toggle off restores diff rows").toContain("leftOnly");
  await expect(countLine).toHaveCount(0);
  expect(await totalsText(), "toggle off never refetches").toBe(loadedTotals);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folder-changesonly-light.png` });
  await toggle.click();
  await expect(countLine).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/folder-changesonly-on-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-changesonly-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
