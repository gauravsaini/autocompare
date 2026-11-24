/**
 * Wave-17 folder row-depth proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine (`dumped.json`
 * served with real paging semantics by `src/ipc/client.ts`) and proves:
 *  - every rendered row carries `folder-row-<index>` with
 *    `data-status` (one of same|different|leftOnly|rightOnly|newerLeft|
 *    newerRight) and a non-empty `data-path`;
 *  - directory rows expose a `folder-collapse-<dirIndex>` toggle that
 *    hides/shows descendants client-side (existing local behavior);
 *  - the `folder-filter-all/diffs/same` control filters the LOADED rows
 *    client-side (same = status same; diffs = everything else) with no
 *    extra IPC (totals label unchanged), diffs hides every same row, and
 *    the buttons show visible counts;
 *  - light + dark PNGs land in `/tmp/ui-17/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). Does not
 * touch `folder-real.spec.ts` or any existing spec.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-17";
const STATUSES = ["same", "different", "leftOnly", "rightOnly", "newerLeft", "newerRight"] as const;

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

test("folder rows: testids + statuses, diffs/same filter with counts, collapse hides descendants", async ({
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

  // Rows render with per-row testids, statuses, and paths.
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  const statuses = await rowStatuses(page);
  expect(statuses.length).toBeGreaterThan(0);
  for (const s of statuses) {
    expect(STATUSES as readonly string[], `known status (${s})`).toContain(s);
  }
  const paths = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  for (const p of paths) {
    expect(p.length, "every row carries a non-empty data-path").toBeGreaterThan(0);
  }
  // Engine sort order: first node is the d00 directory, left-only.
  await expect(rows.first()).toHaveAttribute("data-path", "d00");
  await expect(rows.first()).toHaveAttribute("data-status", "leftOnly");

  // Directory rows expose collapse toggles (existing local behavior).
  const collapse0 = page.getByTestId("folder-collapse-0");
  await expect(collapse0).toBeVisible();
  const descendant = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  await expect(descendant).toHaveCount(1);
  const beforeCollapse = await rows.count();
  await collapse0.click();
  await expect(descendant).toHaveCount(0);
  expect(await rows.count(), "collapsing d00 hides its descendants").toBeLessThan(beforeCollapse);
  await collapse0.click();
  await expect(descendant).toHaveCount(1);
  expect(await rows.count(), "expanding d00 restores its descendants").toBe(beforeCollapse);

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
  expect(loadedTotals).toMatch(/\d+\s*\/\s*\d+\s*entries/);
  const mixed = await rowStatuses(page);
  expect(mixed, "loaded windows contain diff rows").toContain("leftOnly");
  expect(mixed, "loaded windows contain same rows after load-more").toContain("same");

  // Filter control exists with visible counts, and filters client-side
  // (totals label = loaded/total, unchanged by the filter => no refetch).
  const allBtn = page.getByTestId("folder-filter-all");
  const diffsBtn = page.getByTestId("folder-filter-diffs");
  const sameBtn = page.getByTestId("folder-filter-same");
  await expect(allBtn).toBeVisible();
  await expect(diffsBtn).toBeVisible();
  await expect(sameBtn).toBeVisible();
  await expect(allBtn).toContainText(/All \(\d+\)/);
  await expect(diffsBtn).toContainText(/Diffs \(\d+\)/);
  await expect(sameBtn).toContainText(/Same \([1-9]\d*\)/);

  await diffsBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const diffStatuses = await rowStatuses(page);
  expect(diffStatuses.length).toBeGreaterThan(0);
  for (const s of diffStatuses) {
    expect(s, "diffs filter hides every same row").not.toBe("same");
  }
  expect(await totalsText(), "filtering never refetches (totals unchanged)").toBe(loadedTotals);

  await sameBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const sameStatuses = await rowStatuses(page);
  expect(sameStatuses.length).toBeGreaterThan(0);
  for (const s of sameStatuses) {
    expect(s, "same filter shows only same rows").toBe("same");
  }
  expect(await totalsText(), "filtering never refetches (totals unchanged)").toBe(loadedTotals);

  await allBtn.click();
  const allStatuses = await rowStatuses(page);
  expect(allStatuses, "all filter restores both verdicts").toContain("same");
  expect(allStatuses).toContain("leftOnly");

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folder-rows-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-rows-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
