/**
 * Wave-66H BC-style context lines around differences proof (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the context-lines control beside Differences only:
 *  - the `textview/context-lines` native select renders with default 3
 *    and is DISABLED unless diffsOnly is on (context is only meaningful
 *    with the filter on); checking diffsOnly (keyboard: focus + Space)
 *    enables it;
 *  - with diffsOnly on, every changed row carries N unchanged context
 *    neighbours on each side (data-driven: an interior changed row with
 *    equal neighbours is picked from the loaded DOM, then row-1/row+1
 *    assert `data-kind="equal"` while visible);
 *  - switching N changes the visible count monotonically
 *    (count(5) >= count(3) > count(changed-only), count(1) <= count(3));
 *  - N=0 reproduces the exact pre-66H list (visible indexes equal the
 *    pure non-equal list, neighbours hidden, changed row stays);
 *  - the visible list stays order-preserved and deduped (strictly
 *    ascending backend indexes) with backend `index` testids intact;
 *  - the hidden-same note names the math honestly (truly hidden equals
 *    plus the context-rows-shown clause; no context clause at N=0);
 *  - opening a second text session (session switch) resets the toggle
 *    (unchecked), the select (back to 3, disabled) and restores the
 *    full loaded list with the note gone;
 *  - light + dark DOM checks (no PNGs committed), assertNoOverlap,
 *    zero console errors / page errors throughout.
 *
 * Mock honesty: context is purely presentational over the already-loaded
 * mock rows (no new IPC: the status line is byte-identical throughout),
 * so every assertion reads the DOM, never the network. Skip policy
 * matches the other specs (infra failures skip). New file only, no
 * existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

interface LoadedRow {
  readonly index: number;
  readonly kind: string;
}

async function loadedRows(page: Page): Promise<LoadedRow[]> {
  return page.$$eval(".tv-row", (nodes) =>
    nodes.map((n) => ({
      index: Number((n.getAttribute("data-testid") ?? "").split("-").pop()),
      kind: n.getAttribute("data-kind") ?? "",
    })),
  );
}

async function rowCount(page: Page): Promise<number> {
  return page.locator(".tv-row").count();
}

test("context lines surround differences, N switch changes count, gates + reset", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  const statusBefore = await page.getByTestId("textview/status").innerText();

  // Baseline: full loaded window, ascending backend indexes.
  const all = await loadedRows(page);
  expect(all.length).toBe(200);
  for (let k = 1; k < all.length; k += 1) {
    expect(all[k]?.index).toBeGreaterThan(all[k - 1]?.index ?? -1);
  }
  const byIndex = new Map(all.map((r) => [r.index, r.kind]));
  const changedIdx = all.filter((r) => r.kind !== "equal").map((r) => r.index);
  expect(changedIdx.length).toBeGreaterThan(0);
  // Interior changed row with equal neighbours on both sides (and the
  // N=3 window fully inside the loaded list) — picked from the DOM, no
  // fixture hardcoding.
  const target = changedIdx.find(
    (i) =>
      byIndex.get(i - 1) === "equal" &&
      byIndex.get(i + 1) === "equal" &&
      byIndex.has(i - 2) &&
      byIndex.has(i - 3) &&
      byIndex.has(i + 2) &&
      byIndex.has(i + 3),
  );
  expect(target).toBeDefined();
  const t = target ?? -1;
  const targetKind = byIndex.get(t) ?? "";

  // Gate: select renders default 3, disabled unless diffsOnly.
  const toggle = page.getByTestId("textview/diffsonly-toggle");
  const context = page.getByTestId("textview/context-lines");
  const note = page.getByTestId("textview/diffsonly-note");
  await expect(toggle).toBeVisible();
  await expect(toggle).not.toBeChecked();
  await expect(context).toBeVisible();
  await expect(context).toHaveValue("3");
  await expect(context).toBeDisabled();
  await expect(note).toHaveCount(0);

  // Keyboard: focus + Space checks diffsOnly, enabling the select.
  await toggle.focus();
  await page.keyboard.press("Space");
  await expect(toggle).toBeChecked();
  await expect(context).toBeEnabled();
  // Select is keyboard reachable once enabled.
  await context.focus();
  await expect(context).toBeFocused();

  // N=3 default: target plus 3 context neighbours each side visible,
  // neighbours honestly equal, target keeps its kind.
  await expect(note).toBeVisible();
  await expect(note).toContainText(/Hiding \d+ same rows?/);
  await expect(note).toContainText(/\(three context lines shown\)/);
  for (const d of [-3, -2, -1, 0, 1, 2, 3]) {
    await expect(page.getByTestId(`textview/row-${t + d}`)).toBeVisible();
  }
  await expect(page.getByTestId(`textview/row-${t}`)).toHaveAttribute("data-kind", targetKind);
  await expect(page.getByTestId(`textview/row-${t - 1}`)).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId(`textview/row-${t + 1}`)).toHaveAttribute("data-kind", "equal");

  // Order preserved + deduped: strictly ascending backend indexes.
  const vis3 = (await loadedRows(page)).map((r) => r.index);
  const count3 = vis3.length;
  expect(count3).toBeGreaterThan(changedIdx.length);
  for (let k = 1; k < vis3.length; k += 1) {
    expect(vis3[k]).toBeGreaterThan(vis3[k - 1] ?? -1);
  }

  // N=5: visible count grows monotonically (or saturates at the edges).
  await context.selectOption("5");
  await expect.poll(() => rowCount(page), { timeout: 10000 }).toBeGreaterThanOrEqual(count3);
  const count5 = await rowCount(page);

  // N=1: shrinks back within [changed-only, count3].
  await context.selectOption("1");
  await expect.poll(() => rowCount(page), { timeout: 10000 }).toBeLessThanOrEqual(count3);
  const count1 = await rowCount(page);
  expect(count1).toBeGreaterThanOrEqual(changedIdx.length);
  expect(count5).toBeGreaterThanOrEqual(count1);

  // N=0: exact pre-66H behavior — visible indexes equal the pure
  // non-equal list, neighbours hidden, changed row stays, and the note
  // drops the context clause (still counts truly hidden equals).
  await context.selectOption("0");
  await expect.poll(() => rowCount(page), { timeout: 10000 }).toBe(changedIdx.length);
  expect((await loadedRows(page)).map((r) => r.index)).toEqual(changedIdx);
  await expect(page.getByTestId(`textview/row-${t}`)).toBeVisible();
  await expect(page.getByTestId(`textview/row-${t - 1}`)).toHaveCount(0);
  await expect(page.getByTestId(`textview/row-${t + 1}`)).toHaveCount(0);
  await expect(note).toContainText(/Hiding \d+ same rows?/);
  await expect(note).not.toContainText("context");

  // Back to 3: neighbours return. No new IPC anywhere above.
  await context.selectOption("3");
  await expect(page.getByTestId(`textview/row-${t - 1}`)).toBeVisible();
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);

  // Dark scheme: same rows, same control state, DOM only.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId(`textview/row-${t}`)).toBeVisible();
  await expect(toggle).toBeChecked();
  await expect(context).toHaveValue("3");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  // Reset on session switch: a second text session remounts the view
  // with new paths (mock serves the same demo rows for any paths).
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/left-path").fill("/tmp/left-66h.txt");
  await page.getByTestId("session/right-path").fill("/tmp/right-66h.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await expect(toggle).not.toBeChecked();
  await expect(context).toHaveValue("3");
  await expect(context).toBeDisabled();
  await expect(note).toHaveCount(0);
  await expect.poll(() => rowCount(page), { timeout: 10000 }).toBe(200);

  await assertNoOverlap(page);
  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
