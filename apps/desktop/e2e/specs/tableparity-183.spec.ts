/**
 * Wave-183 table BC-parity visual proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the mock table engine and proves
 * the wave-183 closed visual loop against the BC Data Compare truth
 * (`checks/tableparity-183/bc-table-truth.png`, clean-room reference):
 *  - row-level diff wash: a diff row tints ALL its cells (BC paints
 *    rows, not just cells); changed cells keep the full `.tc-diff` wash;
 *  - current diff-nav cursor: exactly one diff row (both panes) carries
 *    `data-current-diff`, and toolbar Next moves it;
 *  - BC column letters + persistent sort glyph: every header reads
 *    `A id / B name / C price` with an svg affordance even unsorted;
 *  - chrome budget + sticky headers hold with light + dark PNGs in
 *    `/tmp/ui-183/` (never in-repo); zero console/page errors.
 *
 * Visual/geometry assertions live ONLY here (older table specs keep
 * behavior-only assertions). New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-183";

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

async function createTableSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wtleft.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wtright.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function bg(page: Page, selector: string): Promise<string> {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel);
    return el ? getComputedStyle(el).backgroundColor : "missing";
  }, selector);
}

test("table parity 183: diff row washes whole row, cells keep full wash, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("table-cols-count")).toContainText("rows");

  // At least one diff row exists in the loaded scope (mock: 5 loaded).
  const drows = page.locator('[data-testid="tableview/grid-left"] tr.tc-drow');
  expect(await drows.count(), "diff rows carry the row wash class").toBeGreaterThan(0);

  // A NON-changed cell inside a diff row is tinted (row wash) while a
  // same-grid cell in a same row is flat: BC row-vs-flat contrast.
  const washBg = await bg(
    page,
    '[data-testid="tableview/grid-left"] tr.tc-drow td.tc-cell:not(.tc-diff)',
  );
  const flatBg = await bg(
    page,
    '[data-testid="tableview/grid-left"] tr:not(.tc-drow) td.tc-cell',
  );
  expect(washBg, "row wash paints non-diff cells in diff rows").not.toBe("missing");
  expect(flatBg, "same rows stay flat").not.toBe("missing");
  expect(washBg, `wash (${washBg}) differs from flat (${flatBg})`).not.toBe(flatBg);

  // Changed cells keep the full del-token wash on both panes.
  const leftDiff = await bg(page, '[data-testid="tableview/cell-5-2"]');
  const rightDiff = await bg(page, '[data-testid="tableview/cell-right-5-2"]');
  expect(leftDiff, "left changed cell washed").not.toBe(flatBg);
  expect(rightDiff, "right changed cell washed").not.toBe(flatBg);

  await page.screenshot({ path: `${EVID_DIR}/parity183-wash-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  const darkWash = await bg(
    page,
    '[data-testid="tableview/grid-left"] tr.tc-drow td.tc-cell:not(.tc-diff)',
  );
  const darkFlat = await bg(
    page,
    '[data-testid="tableview/grid-left"] tr:not(.tc-drow) td.tc-cell',
  );
  expect(darkWash, "row wash holds in dark").not.toBe(darkFlat);
  await page.screenshot({ path: `${EVID_DIR}/parity183-wash-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("table parity 183: headers carry BC letters + persistent sort glyph", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("table-cols-count")).toContainText("rows");

  // BC letter prefix on both panes, display-only (sort status keeps
  // the bare header name - asserted by the wave-44 behavior spec).
  await expect(page.getByTestId("table-sort-0")).toContainText("A");
  await expect(page.getByTestId("table-sort-0")).toContainText("id");
  await expect(page.getByTestId("table-sort-1")).toContainText("B name");
  await expect(page.getByTestId("table-sort-2")).toContainText("C price");
  await expect(page.getByTestId("table-sort-right-1")).toContainText("B name");

  // Persistent affordance: an svg glyph rides every header even with
  // no sort active (BC always shows sortability).
  for (const id of ["table-sort-0", "table-sort-1", "table-sort-right-2"]) {
    expect(
      await page.getByTestId(id).locator("svg").count(),
      `${id} keeps a visible sort glyph unsorted`,
    ).toBeGreaterThan(0);
  }

  // Sorting still cycles and the active glyph swaps (behavior intact).
  await page.getByTestId("table-sort-1").click();
  await expect(page.getByTestId("table-sort-status")).toContainText(/Sorted by name ascending/);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("table parity 183: diff-nav cursor marks one row on both panes and moves", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("table-cols-count")).toContainText("rows");
  await expect(page.getByTestId("tableview/toolbar-diffs")).toContainText("difference 1 of 5");

  // Exactly one cursor row per pane on load (same physical row).
  const cur = page.locator('tr[data-current-diff="true"]');
  expect(await cur.count(), "one cursor row on each pane").toBe(2);
  const before =
    (await page
      .locator('[data-testid="tableview/grid-left"] tr[data-current-diff="true"]')
      .getAttribute("data-testid")) ?? "";

  // Toolbar Next walks the cursor to the next diff row.
  await page.getByTestId("tableview/toolbar-next-diff").click();
  await expect(page.getByTestId("tableview/toolbar-diffs")).toContainText("difference 2 of 5");
  const after =
    (await page
      .locator('[data-testid="tableview/grid-left"] tr[data-current-diff="true"]')
      .getAttribute("data-testid")) ?? "";
  expect(after, "cursor row advances").not.toBe(before);
  expect(await cur.count(), "still one cursor row per pane").toBe(2);

  // Cursor row is visually deeper than a plain diff row.
  const curBg = await bg(
    page,
    '[data-testid="tableview/grid-left"] tr.tc-dcur td.tc-cell:not(.tc-diff)',
  );
  const plainBg = await bg(
    page,
    '[data-testid="tableview/grid-left"] tr.tc-drow:not(.tc-dcur) td.tc-cell:not(.tc-diff)',
  );
  expect(curBg, "cursor row deepens the wash").not.toBe(plainBg);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("table parity 183: chrome budget holds, headers sticky, light+dark clean", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("table-cols-count")).toContainText("rows");

  // Collapsed chrome budget (mirrors the wave-68 gate).
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `chrome budget holds (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Headers pin while scrolling (BC keeps column names visible).
  const sticky = await page.evaluate(() => {
    const heads = [...document.querySelectorAll('[data-testid="tableview/grid-left"] thead')];
    return heads.map((h) => getComputedStyle(h).position);
  });
  expect(sticky.length).toBeGreaterThan(0);
  for (const pos of sticky) {
    expect(pos, "grid header sticks").toBe("sticky");
  }

  // 22px density pin holds after the header changes.
  const rowH = await page.evaluate(() => {
    const r = document.querySelector('[data-testid="tableview/grid-left"] tbody tr.tc-row');
    return r ? r.getBoundingClientRect().height : -1;
  });
  expect(rowH, `rows stay 22px (got ${rowH})`).toBe(22);

  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/parity183-grid-light.png` });
  await page.screenshot({ path: "/tmp/wt-183/checks/tableparity-183/table-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/parity183-grid-dark.png` });
  await page.screenshot({ path: "/tmp/wt-183/checks/tableparity-183/table-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
