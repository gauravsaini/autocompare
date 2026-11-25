/**
 * Wave-L3A folder/table/hex/sync hunk-nav wiring (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Proves the BC unified-toolbar parity slice: App `handleViewCommand`
 * ("next"/"prev") forwards per active kind via the view bus, and each
 * view owns its follow-through over the EXISTING compare IPC (no
 * engine change, no new IPC):
 *  - folder steps the file-row selection over the filtered visible
 *    files order (Wave-228 listener);
 *  - table walks the loaded diff rows via the existing toolbar
 *    `stepDiff` path (Wave-180 listener);
 *  - hex walks differing offsets via the existing `hex_get_page`
 *    windows (Wave-170 listener);
 *  - sync steps the planned diff rows (Wave-180/228 listener);
 *  - media stays honestly parked (summary view: score + metrics, no
 *    cursor — bus listeners are kind-scoped no-ops, never a crash).
 *
 * Top-bar buttons stay disabled with the parked title outside
 * text/merge (toolbar enablement belongs to a later wave — this spec
 * pins NO visual change, only bus-level wiring). Skip policy matches
 * the other specs (infra failures skip). New file only.
 */
import { expect, test, type Page } from "@playwright/test";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function trackErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewTestid: string,
): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestid)).toBeVisible({ timeout: 15000 });
}

async function busCmd(page: Page, cmd: "prev" | "next", viewKind: string): Promise<void> {
  await page.evaluate(
    ({ c, k }) => {
      window.dispatchEvent(
        new CustomEvent("autocompare:view-command", { detail: { cmd: c, viewKind: k } }),
      );
    },
    { c: cmd, k: viewKind },
  );
}

test("L3A folder: bus next/prev steps file selection, foreign kind isolated", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "folder", "/tmp/l3a-folder-left", "/tmp/l3a-folder-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 15000 })
    .toBeGreaterThan(0);

  // Top bar stays parked (NO visual change this wave).
  await expect(page.getByTestId("toolbar/action-next-hunk")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-next-hunk")).toHaveAttribute(
    "title",
    /needs view wiring in L3/,
  );

  // First next selects the first visible file.
  await busCmd(page, "next", "folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"][data-selected="true"]').count(), {
      timeout: 10000,
    })
    .toBe(1);
  const first = await page
    .locator('[data-testid^="folder-row-"][data-selected="true"]')
    .first()
    .getAttribute("data-path");

  // Second next advances (many mock files differ, so it must move).
  await busCmd(page, "next", "folder");
  await expect
    .poll(
      async () =>
        page
          .locator('[data-testid^="folder-row-"][data-selected="true"]')
          .first()
          .getAttribute("data-path"),
      { timeout: 10000 },
    )
    .not.toBe(first);

  // Prev walks back toward the first.
  await busCmd(page, "prev", "folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"][data-selected="true"]').count(), {
      timeout: 10000,
    })
    .toBe(1);

  // Foreign-kind dispatch never leaks across views.
  const before = await page
    .locator('[data-testid^="folder-row-"][data-selected="true"]')
    .first()
    .getAttribute("data-path");
  await busCmd(page, "next", "table");
  await page.waitForTimeout(300);
  const after = await page
    .locator('[data-testid^="folder-row-"][data-selected="true"]')
    .first()
    .getAttribute("data-path");
  expect(after).toBe(before);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3A table: bus next/prev walks diff rows, foreign kind isolated", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "table", "/tmp/l3a-table-left.csv", "/tmp/l3a-table-right.csv", "view-table");
  const diffs = page.getByTestId("tableview/toolbar-diffs");
  await expect(diffs).toContainText(/difference \d+ of \d+/, { timeout: 15000 });
  const boot = (await diffs.textContent()) ?? "";

  // Next advances the diff cursor (mock fixture holds 100+ diffs).
  await busCmd(page, "next", "table");
  await expect.poll(async () => diffs.textContent(), { timeout: 10000 }).not.toBe(boot);

  // Prev walks back.
  const stepped = (await diffs.textContent()) ?? "";
  await busCmd(page, "prev", "table");
  await expect.poll(async () => diffs.textContent(), { timeout: 10000 }).not.toBe(stepped);

  // Foreign-kind dispatch is an honest no-op here, never a crash.
  const before = (await diffs.textContent()) ?? "";
  await busCmd(page, "next", "folder");
  await page.waitForTimeout(300);
  expect(await diffs.textContent()).toBe(before);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3A hex: bus next lands on the differing offset, prev parks honestly", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "hex", "/tmp/l3a-hex-left.bin", "/tmp/l3a-hex-right.bin", "view-hex");
  const status = page.getByTestId("hex-diff-status");
  await expect(status).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("hexview/byte-0")).toBeVisible({ timeout: 15000 });

  // Mock 64-byte fixture differs at offset 8: next lands there.
  await busCmd(page, "next", "hex");
  await expect(status).toContainText(/difference at offset 8/, { timeout: 10000 });
  await expect(page.getByTestId("hex-detail-flag")).toContainText("diff");

  // Offset 8 is the first difference: prev parks honestly, never crashes.
  await busCmd(page, "prev", "hex");
  await expect(status).toContainText(/no earlier differences/, { timeout: 10000 });

  // Foreign-kind dispatch never leaks into hex.
  await busCmd(page, "next", "sync");
  await page.waitForTimeout(300);
  await expect(status).toContainText(/no earlier differences/);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3A sync: bus next/prev steps plan rows, foreign kind isolated", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "sync", "/tmp/l3a-sync-left", "/tmp/l3a-sync-right", "view-sync");
  await expect
    .poll(async () => page.getByTestId("syncview/row").count(), { timeout: 15000 })
    .toBeGreaterThan(0);

  // First next selects the first planned step.
  await busCmd(page, "next", "sync");
  await expect
    .poll(async () => page.locator('[data-testid="syncview/row"][data-selected="true"]').count(), {
      timeout: 10000,
    })
    .toBe(1);
  const first = await page
    .locator('[data-testid="syncview/row"][data-selected="true"]')
    .first()
    .getAttribute("data-path");

  // Second next advances (mock plan holds 80+ steps, so it must move).
  await busCmd(page, "next", "sync");
  await expect
    .poll(
      async () =>
        page
          .locator('[data-testid="syncview/row"][data-selected="true"]')
          .first()
          .getAttribute("data-path"),
      { timeout: 10000 },
    )
    .not.toBe(first);

  // Prev keeps exactly one row selected (walks back, never a crash).
  await busCmd(page, "prev", "sync");
  await expect
    .poll(async () => page.locator('[data-testid="syncview/row"][data-selected="true"]').count(), {
      timeout: 10000,
    })
    .toBe(1);

  // Foreign-kind dispatch never leaks across views.
  const before = await page
    .locator('[data-testid="syncview/row"][data-selected="true"]')
    .first()
    .getAttribute("data-path");
  await busCmd(page, "next", "hex");
  await page.waitForTimeout(300);
  const after = await page
    .locator('[data-testid="syncview/row"][data-selected="true"]')
    .first()
    .getAttribute("data-path");
  expect(after).toBe(before);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3A media: bus next/prev is an honest no-op, never a crash", async ({ page }) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "media", "/tmp/l3a-m-left.png", "/tmp/l3a-m-right.png", "view-media");
  await expect(page.getByTestId("mediaview/summary")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("mediaview/score")).toContainText(/bytes differ/);

  // Media is a summary view (no cursor): bus nav parks honestly.
  await busCmd(page, "next", "media");
  await busCmd(page, "prev", "media");
  await page.waitForTimeout(300);
  await expect(page.getByTestId("mediaview/summary")).toBeVisible();
  await expect(page.getByTestId("mediaview/score")).toContainText(/bytes differ/);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
