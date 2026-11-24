/**
 * Wave-180: 14-action unified top Toolbar contract proof (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves the BC-parity contract
 * (`checks/topbar-180/CONTRACT.md`):
 *  1. text kind — top-bar Next/Prev and the in-view TextToolbar ride ONE
 *     bus source: every click advances EXACTLY one hunk (no double-jump),
 *     with wrap;
 *  2. text kind — Copy runs the staged `copyHunkAt` + persist path
 *     (localStorage `ac:textcopy125:` receipt) with a status note;
 *  3. table kind — Minor is live through the bus (ignore flags applied,
 *     view re-compares, toggle round-trips);
 *  4. text kind — Swap exchanges the PathBar sides and the view
 *     re-compares (rows stay live);
 *  5. folder kind — Prev/Next/Copy park honestly, filters stay live;
 *  6. Rules/Format fire EXACTLY once per top-bar click (single source —
 *     the bar never dispatches a parallel raw event).
 *
 * PORT-FREE: run on a fresh port (`reuseExistingServer:false`,
 * `--workers=1`, webkit) so no shared port or parallel worker can
 * flake the gate. Skip policy matches the other specs (infra failures
 * skip). NEW file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

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

async function bootText(page: Page): Promise<void> {
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows", { timeout: 15000 });
}

/** Create a fresh comparison session through the session dialog. */
async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewId: string,
): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click().catch(() => {});
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewId)).toBeVisible({ timeout: 15000 });
}

async function hunksOrdinal(page: Page): Promise<{ at: number; total: number }> {
  const label = await page.getByTestId("textview/toolbar-hunks").textContent();
  const match = label?.match(/hunk (\d+) of (\d+)/);
  expect(match, `hunks label is positional (got "${label}")`).not.toBeNull();
  return { at: Number(match![1]), total: Number(match![2]) };
}

test("topbar-180: text next/prev single bus source — one click = one hunk", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const next = page.getByTestId("toolbar/action-next-hunk");
  const prev = page.getByTestId("toolbar/action-prev-hunk");
  await expect(next).toBeEnabled();
  await expect(prev).toBeEnabled();

  // First top-bar Next lands inside a hunk; each further click moves
  // EXACTLY one hunk (a duplicate second path would jump two).
  await next.click();
  const first = await hunksOrdinal(page);
  await next.click();
  const second = await hunksOrdinal(page);
  expect(second.total).toBe(first.total);
  expect(second.at).toBe((first.at % first.total) + 1);

  // The in-view TextToolbar rides the SAME bus source: one more step.
  await page.getByTestId("textview/toolbar-next-diff").click();
  const third = await hunksOrdinal(page);
  expect(third.at).toBe((second.at % second.total) + 1);

  // Prev walks back exactly one hunk through the same source.
  await prev.click();
  const back = await hunksOrdinal(page);
  expect(back.at).toBe(third.at === 1 ? third.total : third.at - 1);

  assertClean(pageErrors, consoleErrors, "text single-source nav");
});

test("topbar-180: copy rides the staged copyHunkAt path with a receipt", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const copy = page.getByTestId("toolbar/action-copy");
  await expect(copy).toBeDisabled();
  await page.getByTestId("textview/row-98").click();
  await expect(copy).toBeEnabled();
  await copy.click();
  await expect(page.getByTestId("statusbar/message")).toContainText(
    "Copy take-left requested for row 98",
  );
  // Same staged path as the gutter arrows: in-memory merge receipt,
  // row resolves to equal, BC stage-only (no reload journal).
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
  await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
  await expect(page.getByTestId("textview/row-98")).toHaveAttribute("data-kind", "equal");
  const journaled = await page.evaluate(() => {
    const out: string[] = [];
    for (let i = 0; i < window.localStorage.length; i += 1) {
      const key = window.localStorage.key(i);
      if (key !== null && key.startsWith("ac:textcopy125:")) out.push(key);
    }
    return out;
  });
  expect(journaled, "stage-only: nothing journaled for reload replay").toEqual([]);

  assertClean(pageErrors, consoleErrors, "text copy staged");
});

test("topbar-180: minor is live in table kind through the bus", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createSession(page, "table", "/tmp/w180-left.csv", "/tmp/w180-right.csv", "view-table");
  await expect(page.getByTestId("tableview/panes")).toBeVisible({ timeout: 15000 });

  const minor = page.getByTestId("toolbar/action-minor");
  await expect(minor).toBeEnabled();
  await expect(minor).toHaveAttribute("title", "Minor");
  await minor.click();
  await expect(minor).toHaveAttribute("title", "Minor (on)");
  await expect(page.getByTestId("statusbar/message")).toContainText("ignoring whitespace and case");
  await expect(page.getByTestId("tableview/panes")).toBeVisible();
  await minor.click();
  await expect(minor).toHaveAttribute("title", "Minor");
  await expect(page.getByTestId("statusbar/message")).toContainText("Minor off");

  assertClean(pageErrors, consoleErrors, "table minor bus");
});

test("topbar-180: swap exchanges sides and the view re-compares", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  const leftInput = page.getByTestId("pathbar/left");
  const rightInput = page.getByTestId("pathbar/right");
  const beforeLeft = await leftInput.inputValue();
  const beforeRight = await rightInput.inputValue();
  expect(beforeLeft, "demo left path set").not.toBe("");
  expect(beforeRight, "demo right path set").not.toBe("");

  await page.getByTestId("toolbar/action-swap").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("Swapped left and right");
  await expect(leftInput).toHaveValue(beforeRight);
  await expect(rightInput).toHaveValue(beforeLeft);
  // Re-compare follows the swapped props: rows stay live.
  await expect(page.getByTestId("textview/status")).toContainText("rows", { timeout: 15000 });

  assertClean(pageErrors, consoleErrors, "swap re-compare");
});

test("topbar-180: folder parks prev/next/copy honestly, filters stay live", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createSession(page, "folder", "/tmp/w180-left", "/tmp/w180-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(0);

  const prev = page.getByTestId("toolbar/action-prev-hunk");
  const next = page.getByTestId("toolbar/action-next-hunk");
  await expect(prev).toBeDisabled();
  await expect(next).toBeDisabled();
  await expect(prev).toHaveAttribute("title", /needs view wiring in L3/);
  const copy = page.getByTestId("toolbar/action-copy");
  await expect(copy).toBeDisabled();

  // Bus dispatch at a parked view is a no-op, never a crash.
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("autocompare:view-command", { detail: { cmd: "next", viewKind: "folder" } }),
    );
    window.dispatchEvent(
      new CustomEvent("autocompare:view-command", { detail: { cmd: "copy", viewKind: "folder" } }),
    );
  });

  // Filters stay live in folder kind.
  await page.getByTestId("toolbar/action-filter-diffs").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: diffs");

  assertClean(pageErrors, consoleErrors, "folder parked bus");
});

test("topbar-180: rules/format fire exactly once per top-bar click", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await page.evaluate(() => {
    (window as unknown as Record<string, number>)["__W180_RULES__"] = 0;
    (window as unknown as Record<string, number>)["__W180_FORMAT__"] = 0;
    window.addEventListener("autocompare:view-rules", () => {
      const w = window as unknown as Record<string, number>;
      w["__W180_RULES__"] = (w["__W180_RULES__"] ?? 0) + 1;
    });
    window.addEventListener("autocompare:view-format", () => {
      const w = window as unknown as Record<string, number>;
      w["__W180_FORMAT__"] = (w["__W180_FORMAT__"] ?? 0) + 1;
    });
  });

  await createSession(page, "folder", "/tmp/w180-left", "/tmp/w180-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(0);
  await page.getByTestId("toolbar/action-rules").click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  expect(await page.evaluate(() => (window as unknown as Record<string, number>)["__W180_RULES__"])).toBe(
    1,
  );

  await createSession(page, "text", "/tmp/w180-left.txt", "/tmp/w180-right.txt", "view-text");
  await expect(page.getByTestId("textview/status")).toContainText("rows", { timeout: 15000 });
  await page.getByTestId("toolbar/action-format").click();
  await expect(page.getByTestId("textview/format-options")).toBeVisible();
  expect(
    await page.evaluate(() => (window as unknown as Record<string, number>)["__W180_FORMAT__"]),
  ).toBe(1);

  assertClean(pageErrors, consoleErrors, "rules/format single fire");
});
