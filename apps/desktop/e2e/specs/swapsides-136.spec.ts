/**
 * Wave-136 swap sides (BC parity, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text pair
 * (File 1 = left fixture, File 2 = right fixture, mock `text_compare`
 * over `dumped.json`) and proves the BC Swap (left<->right exchange):
 *  - toolbar `toolbar/action-swap` ("Swap sides") swaps the ACTIVE tab
 *    left/right paths (base ancestor untouched — text has none);
 *  - panes + gutter + ruler follow the swap, hunk counts identical
 *    before/after (`textview/status` same/changed/left-only/right-only
 *    triple + `hunk-copy-left-*` gutter buttons + `ruler/row-*` markers);
 *  - double swap restores the exact original paths;
 *  - the tab dirty state survives the swap (no data loss, no save
 *    dialog, no close prompt); blank tabs are a no-op, never an error;
 *  - zero console errors / page errors throughout.
 *
 * No new IPC: every assertion reads the DOM. Skip policy matches the
 * other specs (infra failures skip). NEW file only, no existing spec
 * touched.
 */
import { expect, test, type Page } from "@playwright/test";

async function pathValue(page: Page, testId: string): Promise<string> {
  return page.getByTestId(testId).inputValue();
}

async function statusCounts(page: Page): Promise<string> {
  const text = (await page.getByTestId("textview/status").textContent()) ?? "";
  const m = text.match(/(\d+) same · (\d+) changed · (\d+) left-only · (\d+) right-only/);
  return m ? `${m[1]}/${m[2]}/${m[3]}/${m[4]}` : `UNPARSED:${text.slice(0, 120)}`;
}

async function statusFull(page: Page): Promise<string> {
  return ((await page.getByTestId("textview/status").textContent()) ?? "").trim();
}

async function hunkCount(page: Page): Promise<number> {
  return page.locator('[data-testid^="hunk-copy-left-"]').count();
}

async function rulerCount(page: Page): Promise<number> {
  return page.locator('[data-testid^="ruler/row-"]').count();
}

async function dirtyCount(page: Page): Promise<number> {
  return page.getByTestId("tabbar").locator('[data-testid^="tab-dirty-"]').count();
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

async function bootDemo(page: Page): Promise<{ left: string; right: string }> {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/status")).toContainText("changed", { timeout: 15000 });
  const left = await pathValue(page, "pathbar/left");
  const right = await pathValue(page, "pathbar/right");
  expect(left, "demo left path non-blank").not.toBe("");
  expect(right, "demo right path non-blank").not.toBe("");
  expect(left, "left != right before swap").not.toBe(right);
  return { left, right };
}

test("swapsides-136: swap exchanges sides, hunk counts identical", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  const { left, right } = await bootDemo(page);
  const swap = page.getByTestId("toolbar/action-swap");
  await expect(swap).toBeVisible();
  await expect(swap).toBeEnabled();
  await expect(swap).toHaveAttribute("title", "Swap sides");

  const countsBefore = await statusCounts(page);
  expect(countsBefore, "status counts parse before swap").not.toMatch(/^UNPARSED/);
  const fullBefore = await statusFull(page);
  const hunksBefore = await hunkCount(page);
  expect(hunksBefore, "gutter hunks present before swap").toBeGreaterThan(0);
  const rulerBefore = await rulerCount(page);

  await swap.click();

  // Panes follow: pathbar values exchange exactly.
  await expect.poll(() => pathValue(page, "pathbar/left"), { timeout: 10000 }).toBe(right);
  await expect.poll(() => pathValue(page, "pathbar/right"), { timeout: 10000 }).toBe(left);
  // Let the view reload settle on the swapped pair (mock IPC resolves
  // in ms; settle by time, then pin counts + hunks + ruler identical and
  // the status path suffix exchanged: "left vs right" -> "right vs left").
  await page.waitForTimeout(1000);
  await expect(page.getByTestId("textview/status")).toBeVisible();

  // Hunk counts identical across the swap; path suffix exchanged.
  const fullAfter = await statusFull(page);
  if (fullBefore.includes(`${left} vs ${right}`)) {
    expect(fullAfter, "status text identical except exchanged path suffix").toBe(
      fullBefore.replace(`${left} vs ${right}`, `${right} vs ${left}`),
    );
  } else {
    // Suffix form differs from raw input values (e.g. resolved vs
    // display paths): pin the swapped order explicitly instead.
    expect(fullAfter, "status path suffix exchanged after swap").toContain(`${right} vs ${left}`);
  }
  await expect.poll(() => statusCounts(page), { timeout: 15000 }).toBe(countsBefore);
  await expect.poll(() => hunkCount(page), { timeout: 15000 }).toBe(hunksBefore);
  expect(await rulerCount(page), "ruler markers identical after swap").toBe(rulerBefore);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("swapsides-136: double swap restores exact original paths", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  const { left, right } = await bootDemo(page);
  const countsBefore = await statusCounts(page);
  const fullBefore = await statusFull(page);
  const swap = page.getByTestId("toolbar/action-swap");
  await expect(swap).toBeEnabled();

  await swap.click();
  await expect.poll(() => pathValue(page, "pathbar/left"), { timeout: 10000 }).toBe(right);
  await page.waitForTimeout(1000);

  await swap.click();
  await expect.poll(() => pathValue(page, "pathbar/left"), { timeout: 10000 }).toBe(left);
  await expect.poll(() => pathValue(page, "pathbar/right"), { timeout: 10000 }).toBe(right);
  await page.waitForTimeout(1000);
  expect(await statusFull(page), "full status text restored after double swap").toBe(fullBefore);
  await expect.poll(() => statusCounts(page), { timeout: 15000 }).toBe(countsBefore);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});

test("swapsides-136: dirty preserved across swap, no save dialog, blank no-op", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  const { left, right } = await bootDemo(page);
  // Demo boots with fresh (never-compared) inputs: the tab is dirty.
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);

  const swap = page.getByTestId("toolbar/action-swap");
  await swap.click();
  await expect.poll(() => pathValue(page, "pathbar/left"), { timeout: 10000 }).toBe(right);

  // Dirty survives: no data loss, no save dialog, no close prompt.
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  await expect(page.getByTestId("session/dialog")).toHaveCount(0);
  await expect(page.getByTestId("tabbar").locator('[data-testid^="tab-close-prompt-"]')).toHaveCount(
    0,
  );
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Blank tab: swap is a no-op, never an error.
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("text");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect.poll(() => pathValue(page, "pathbar/left"), { timeout: 10000 }).toBe("");
  await expect.poll(() => pathValue(page, "pathbar/right"), { timeout: 10000 }).toBe("");
  await swap.click();
  await expect.poll(() => pathValue(page, "pathbar/left"), { timeout: 10000 }).toBe("");
  await expect.poll(() => pathValue(page, "pathbar/right"), { timeout: 10000 }).toBe("");
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
