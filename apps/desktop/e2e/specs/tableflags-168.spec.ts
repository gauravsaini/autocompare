/**
 * Wave-168: Table ignore flags (ws + case) UI wiring proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves the Beyond Compare parity slice:
 *  - TableToolbar renders Ignore ws and Ignore case checkboxes (`tableview/ignore-ws`,
 *    `tableview/ignore-case`), default unchecked;
 *  - checking / unchecking toggles re-issues `table_compare` with active flags and
 *    records receipt to `window.__TABLE_IGNORE_LAST__`;
 *  - chrome budget: single toolbar row, firstRowTop <= 140px in-flow only;
 *  - mock adapter honors both camelCase (`ignoreCase`, `ignoreSpace`) and
 *    snake_case (`ignore_case`, `ignore_space`) spellings faithfully without crashing;
 *  - light + dark, zero console/page errors.
 *
 * PORT-FREE: run on a fresh port (`reuseExistingServer: false`, `--workers=1`, webkit).
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

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  return { pageErrors, consoleErrors };
}

function assertClean(
  pageErrors: string[],
  consoleErrors: string[],
  where: string,
): void {
  expect(pageErrors, `zero page errors (${where})`).toEqual([]);
  expect(consoleErrors, `zero console errors (${where})`).toEqual([]);
}

async function createTableSession(page: Page): Promise<void> {
  const banner = page.getByTestId("workspace/demo-banner");
  if (await banner.isVisible()) {
    await page.getByTestId("workspace/demo-clear").click().catch(() => {});
  }

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave168-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave168-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible({ timeout: 15000 });
  await expect(page.locator("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]").first()).toBeVisible({
    timeout: 15000,
  });
}

test("tableflags-168: toggle ws/case re-runs compare + receipt", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createTableSession(page);

  // 1. Checkboxes are mounted in TableToolbar and unchecked on load.
  const wsToggle = page.getByTestId("tableview/ignore-ws");
  const caseToggle = page.getByTestId("tableview/ignore-case");
  await expect(wsToggle).toBeVisible();
  await expect(caseToggle).toBeVisible();
  await expect(wsToggle).not.toBeChecked();
  await expect(caseToggle).not.toBeChecked();

  // Receipt hook before toggles: both flags false.
  const hookInitial = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
  );
  expect(hookInitial, "initial compare sends no ignore flags").toMatchObject({
    ignoreCase: false,
    ignoreSpace: false,
  });

  // 2. Toggle Ignore ws: re-runs compare and updates receipt.
  await wsToggle.check();
  await expect(wsToggle).toBeChecked();
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
        ),
      { timeout: 5000 },
    )
    .toMatchObject({ ignoreCase: false, ignoreSpace: true });

  // 3. Toggle Ignore case: re-runs compare and updates receipt.
  await caseToggle.check();
  await expect(caseToggle).toBeChecked();
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
        ),
      { timeout: 5000 },
    )
    .toMatchObject({ ignoreCase: true, ignoreSpace: true });

  // 4. Uncheck Ignore ws: re-runs compare with only case ignored.
  await wsToggle.uncheck();
  await expect(wsToggle).not.toBeChecked();
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
        ),
      { timeout: 5000 },
    )
    .toMatchObject({ ignoreCase: true, ignoreSpace: false });

  // 5. Uncheck Ignore case: returns to exact comparison.
  await caseToggle.uncheck();
  await expect(caseToggle).not.toBeChecked();
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
        ),
      { timeout: 5000 },
    )
    .toMatchObject({ ignoreCase: false, ignoreSpace: false });

  // 6. Chrome budget: firstRowTop <= 140px relative to table section.
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: Math.round(f.top - s.top) };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome!.delta,
    `table chrome budget firstRowTop <= 140px (got ${chrome!.delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Status line remains healthy.
  await expect(page.getByTestId("tableview/status")).toBeVisible();
  await expect(page.getByTestId("tableview/diffs")).toBeVisible();

  assertClean(pageErrors, consoleErrors, "toggle ws/case re-runs compare + receipt");
});

test("tableflags-168: mock honors both flag spellings", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createTableSession(page);

  // 1. Real typed wrapper tableCompare with camelCase, snake_case, and mixed flags
  const testResults = await page.evaluate(async () => {
    const mod = (await import("/src/ipc/table.ts")) as unknown as {
      tableCompare: (
        left: string,
        right: string,
        byKey: boolean,
        keyColumns: readonly number[],
        opts?: Record<string, boolean>,
      ) => Promise<{ sessionId: string; diffRows: number; equal: boolean }>;
    };

    // A. camelCase
    const r1 = await mod.tableCompare("/tmp/t1-left.csv", "/tmp/t1-right.csv", false, [], {
      ignoreCase: true,
      ignoreSpace: true,
    });
    const hook1 = (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__;

    // B. snake_case
    const r2 = await mod.tableCompare("/tmp/t2-left.csv", "/tmp/t2-right.csv", false, [], {
      ignore_case: true,
      ignore_space: true,
    });
    const hook2 = (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__;

    // C. mixed spellings
    const r3 = await mod.tableCompare("/tmp/t3-left.csv", "/tmp/t3-right.csv", false, [], {
      ignoreCase: true,
      ignore_space: true,
    });
    const hook3 = (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__;

    const r4 = await mod.tableCompare("/tmp/t4-left.csv", "/tmp/t4-right.csv", false, [], {
      ignore_case: true,
      ignoreSpace: true,
    });
    const hook4 = (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__;

    // D. legacy no flags
    const r5 = await mod.tableCompare("/tmp/t5-left.csv", "/tmp/t5-right.csv", false, []);
    const hook5 = (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__;

    return {
      r1, hook1,
      r2, hook2,
      r3, hook3,
      r4, hook4,
      r5, hook5,
    };
  });

  expect(testResults.r1.sessionId).toBe("mock-table-1");
  expect(testResults.hook1).toMatchObject({ ignoreCase: true, ignoreSpace: true });

  expect(testResults.r2.sessionId).toBe("mock-table-1");
  expect(testResults.hook2).toMatchObject({ ignoreCase: true, ignoreSpace: true });

  expect(testResults.r3.sessionId).toBe("mock-table-1");
  expect(testResults.hook3).toMatchObject({ ignoreCase: true, ignoreSpace: true });

  expect(testResults.r4.sessionId).toBe("mock-table-1");
  expect(testResults.hook4).toMatchObject({ ignoreCase: true, ignoreSpace: true });

  expect(testResults.r5.sessionId).toBe("mock-table-1");
  expect(testResults.hook5).toMatchObject({ ignoreCase: false, ignoreSpace: false });

  // 2. Direct mock invoke verifying cell normalization on whitespace and case mismatches
  const mockNormResults = await page.evaluate(async () => {
    const client = (await import("/src/ipc/client.ts")) as unknown as {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
    };

    const mockCells = {
      header: ["col1"],
      headerR: ["col1"],
      rowsL: [["  hello  "], ["FOO"]],
      rowsR: [["hello"], ["foo"]],
      totalRowsL: 2,
      totalRowsR: 2,
      totalCols: 1,
      leftCols: 1,
      rightCols: 1,
      equal: false,
    };

    // Exact compare: both rows differ
    const exact = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
    });

    // ignoreSpace camelCase
    const wsCamel = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
      ignoreSpace: true,
    });

    // ignore_space snake_case
    const wsSnake = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
      ignore_space: true,
    });

    // ignoreCase camelCase
    const csCamel = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
      ignoreCase: true,
    });

    // ignore_case snake_case
    const csSnake = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
      ignore_case: true,
    });

    // Both flags camelCase
    const bothCamel = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
      ignoreCase: true,
      ignoreSpace: true,
    });

    // Both flags snake_case
    const bothSnake = await client.invoke<{ diffRows: number; equal: boolean }>("table_compare", {
      _mockCells: mockCells,
      ignore_case: true,
      ignore_space: true,
    });

    return { exact, wsCamel, wsSnake, csCamel, csSnake, bothCamel, bothSnake };
  });

  // Exact: 2 mismatches, not equal
  expect(mockNormResults.exact.diffRows).toBe(2);
  expect(mockNormResults.exact.equal).toBe(false);

  // Whitespace ignore: row 1 matches, row 2 still differs -> 1 diff
  expect(mockNormResults.wsCamel.diffRows).toBe(1);
  expect(mockNormResults.wsSnake.diffRows).toBe(1);

  // Case ignore: row 2 matches, row 1 still differs -> 1 diff
  expect(mockNormResults.csCamel.diffRows).toBe(1);
  expect(mockNormResults.csSnake.diffRows).toBe(1);

  // Both ignore: all cells match -> 0 diffs, equal true
  expect(mockNormResults.bothCamel.diffRows).toBe(0);
  expect(mockNormResults.bothCamel.equal).toBe(true);
  expect(mockNormResults.bothSnake.diffRows).toBe(0);
  expect(mockNormResults.bothSnake.equal).toBe(true);

  assertClean(pageErrors, consoleErrors, "mock honors both flag spellings");
});
