/**
 * Wave-14 toolbar hunk nav proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 * Wave-18 repair: bucket-agnostic reads for the wave-15 ruler.
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 15 hunks) and proves:
 *  - the toolbar exposes prev-hunk / next-hunk actions
 *    (`toolbar/action-prev-hunk`, `toolbar/action-next-hunk`), enabled for
 *    the text kind while hunks exist;
 *  - hunks exist per the pane itself: the `textview/status` line reports
 *    `644 / 644 rows` with fewer `same` rows than total, and the rendered
 *    pane rows (`textview/row-<index>` with backend `data-kind`) group
 *    into more than one hunk, where a hunk starts at each index whose
 *    kind is not `"equal"` and whose predecessor is `"equal"` (or index
 *    0). Hunk starts NEVER come from the ruler DOM: since wave-15 the
 *    ruler renders <= 400 buckets (`ruler/row-<bucketFirstIndex>` with
 *    worst-kind), so bucket testids are not row indices;
 *  - clicking next-hunk moves the ruler position (`ruler`
 *    `aria-valuenow`) to the first hunk-start strictly greater than the
 *    current one (wrapping to the first start past the last hunk), the
 *    virtual scroller (`textview/virtual`) `scrollTop` moves with it,
 *    the active bucket (`ruler/row-*` with `data-active="true"`)
 *    contains the target row, and the target pane row
 *    (`textview/row-<target>`) carries a non-`"equal"` `data-kind`;
 *    prev-hunk mirrors this (wrap to last, then walk back down);
 *  - light + dark PNGs land in `/tmp/ui-18/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

// Full-pane scan plus a ~15-hunk jump cycle needs headroom over the
// 30s default (webkit can be slow to boot under load).
test.setTimeout(120_000);

const EVID_DIR = "/tmp/ui-18";

/** Backend pane kinds: `"equal"` is background, everything else is a hunk row. */
function isHunkKind(kind: string): boolean {
  return kind !== "equal";
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

/** Ruler position: the `slider`'s `aria-valuenow` (authoritative row index). */
async function rulerValueNow(page: Page): Promise<number> {
  const raw = await page.getByTestId("ruler").getAttribute("aria-valuenow");
  if (raw === null) throw new Error("ruler has no aria-valuenow");
  return Number(raw);
}

/** Sorted first-indices of every rendered ruler bucket. */
async function bucketStarts(page: Page): Promise<number[]> {
  const testids = await page
    .locator('[data-testid^="ruler/row-"]')
    .evaluateAll((els) => els.map((el) => el.getAttribute("data-testid") ?? ""));
  return testids.map((id) => Number(id.replace("ruler/row-", ""))).sort((a, b) => a - b);
}

/** First-index of the bucket carrying `data-active="true"` (exactly one). */
async function activeBucketStart(page: Page): Promise<number> {
  await expect(page.locator('[data-testid^="ruler/row-"][data-active="true"]')).toHaveCount(1);
  const testid = await page
    .locator('[data-testid^="ruler/row-"][data-active="true"]')
    .first()
    .getAttribute("data-testid");
  if (testid === null) throw new Error("no active ruler bucket");
  return Number(testid.replace("ruler/row-", ""));
}

/**
 * Hunk-start row indices from the RENDERED PANE rows (bucket-agnostic).
 * The pane virtualizes past 500 rows, so only the visible window is
 * mounted: this scrolls the `textview/virtual` scroller top-to-bottom in
 * client-height steps, harvesting every `textview/row-<index>`
 * `data-kind` into one index->kind map, then starts a hunk at each
 * non-`"equal"` row whose predecessor is `"equal"` (or index 0) -- the
 * same grouping `App` uses over `rulerRows`, without touching the
 * compressed ruler DOM. Restores the scroller to the top and waits for
 * the ruler position to settle back to 0 before returning.
 */
async function paneHunks(page: Page): Promise<{ starts: number[]; kinds: Map<number, string> }> {
  const byIndex = new Map<number, string>();
  for (let guard = 0; guard < 80; guard += 1) {
    const rows = await page.$$eval('[data-testid^="textview/row-"]', (nodes) =>
      nodes.map((n) => ({
        index: Number((n.getAttribute("data-testid") ?? "").replace("textview/row-", "")),
        kind: n.getAttribute("data-kind") ?? "",
      })),
    );
    for (const row of rows) {
      if (Number.isFinite(row.index) && !byIndex.has(row.index)) byIndex.set(row.index, row.kind);
    }
    const pos = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="textview/virtual"]');
      if (!(el instanceof HTMLElement)) return null;
      return { top: el.scrollTop, height: el.scrollHeight, client: el.clientHeight };
    });
    if (pos === null) throw new Error("no virtual scroller");
    if (pos.top + pos.client >= pos.height - 1) break;
    await page.evaluate(({ top, client }) => {
      const el = document.querySelector('[data-testid="textview/virtual"]');
      if (el instanceof HTMLElement) el.scrollTop = top + client;
    }, { top: pos.top, client: pos.client });
    await page.waitForTimeout(100);
  }
  expect(byIndex.size, "pane scan covers every row of the full diff").toBe(644);
  const sorted = [...byIndex.entries()].sort((a, b) => a[0] - b[0]);
  const starts: number[] = [];
  for (const [index, kind] of sorted) {
    if (!isHunkKind(kind)) continue;
    if (index === 0 || !isHunkKind(byIndex.get(index - 1) ?? "equal")) starts.push(index);
  }
  // Back to the top so jump proofs start from a known position.
  await page.evaluate(() => {
    const el = document.querySelector('[data-testid="textview/virtual"]');
    if (el instanceof HTMLElement) el.scrollTop = 0;
  });
  await expect.poll(async () => rulerValueNow(page), { timeout: 5000 }).toBe(0);
  return { starts, kinds: byIndex };
}

async function paneKind(page: Page, index: number): Promise<string | null> {
  const loc = page.getByTestId(`textview/row-${index}`);
  if ((await loc.count()) === 0) return null;
  return loc.getAttribute("data-kind");
}

async function scrollTop(page: Page): Promise<number> {
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="textview/virtual"]');
    return el instanceof HTMLElement ? el.scrollTop : -1;
  });
}

/** First hunk-start strictly greater than `from`, wrapping to the first. */
function nextStartAfter(starts: readonly number[], from: number): number {
  const next = starts.find((s) => s > from);
  const fallback = starts[0];
  if (next !== undefined) return next;
  if (fallback === undefined) throw new Error("no hunk starts");
  return fallback;
}

/** Last hunk-start strictly less than `from`, wrapping to the last. */
function prevStartBefore(starts: readonly number[], from: number): number {
  for (let i = starts.length - 1; i >= 0; i -= 1) {
    const s = starts[i];
    if (s !== undefined && s < from) return s;
  }
  const fallback = starts[starts.length - 1];
  if (fallback === undefined) throw new Error("no hunk starts");
  return fallback;
}

test("toolbar next/prev hunk jumps advance, wrap, and scroll the text pane", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("ruler")).toBeVisible();
  await expect(page.getByTestId("ruler")).toHaveAttribute("role", "slider");
  const buckets = page.locator('[data-testid^="ruler/row-"]');
  await expect.poll(async () => buckets.count(), { timeout: 15000 }).toBeGreaterThan(0);

  // Load the full comparison so the pane holds all 644 mock rows: every
  // pane-derived hunk-start is then a rendered, reachable row.
  for (let i = 0; i < 10; i += 1) {
    const more = page.getByRole("button", { name: /Load more/ });
    if ((await more.count()) === 0) break;
    await more.first().click();
    await page.waitForTimeout(400);
  }
  await expect(page.getByTestId("textview/virtual")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("textview/status")).toContainText("644 / 644 rows");

  // Hunks exist per the status-line counts (fewer `same` than total).
  const status = (await page.getByTestId("textview/status").textContent()) ?? "";
  const totalMatch = status.match(/(\d+)\s*\/\s*(\d+)\s*rows/);
  const sameMatch = status.match(/(\d+)\s*same/);
  expect(totalMatch, `status reports a row total (got: ${status})`).not.toBeNull();
  expect(Number(totalMatch?.[2]), "status total is 644 rows").toBe(644);
  expect(sameMatch, `status reports a same count (got: ${status})`).not.toBeNull();
  expect(Number(sameMatch?.[1]), "status same count is below the total (hunks exist)").toBeLessThan(644);

  // Hunk starts come from the pane rows, never the bucketed ruler.
  const { starts, kinds } = await paneHunks(page);
  expect(starts.length, "pane rows group into more than one hunk").toBeGreaterThan(1);
  for (const s of starts) {
    const kind = kinds.get(s) ?? "";
    expect(kind, `hunk start row ${s} is changed content`).toMatch(
      /^(changed|leftOnly|rightOnly)$/,
    );
    if (s > 0) {
      expect(kinds.get(s - 1), `row before hunk start ${s} is same`).toBe("equal");
    }
  }

  const nextBtn = page.getByTestId("toolbar/action-next-hunk");
  const prevBtn = page.getByTestId("toolbar/action-prev-hunk");
  await expect(nextBtn).toBeEnabled();
  await expect(prevBtn).toBeEnabled();

  // One jump proof: position + scroll + active bucket + landed content.
  async function expectJumpedTo(target: number, prevTop: number): Promise<void> {
    await expect
      .poll(async () => rulerValueNow(page), { timeout: 10000 })
      .toBe(target);
    expect(await scrollTop(page), `pane scrolls for hunk ${target}`).not.toBe(prevTop);
    const bucketsNow = await bucketStarts(page);
    const active = await activeBucketStart(page);
    const nextBucket = bucketsNow.find((b) => b > active);
    expect(active, `active bucket starts at or before hunk ${target}`).toBeLessThanOrEqual(target);
    if (nextBucket !== undefined) {
      expect(target, `hunk ${target} sits inside the active bucket`).toBeLessThan(nextBucket);
    }
    await expect
      .poll(async () => paneKind(page, target), { timeout: 10000 })
      .toMatch(/^(changed|leftOnly|rightOnly)$/);
  }

  // Next from the top lands on the first hunk-start strictly greater
  // than the current ruler position, and the pane scrolls with it.
  const initial = await rulerValueNow(page);
  const beforeTop = await scrollTop(page);
  await nextBtn.click();
  const first = nextStartAfter(starts, initial);
  await expectJumpedTo(first, beforeTop);

  // Step forward through every remaining hunk one by one; after exactly
  // `starts.length` more next-clicks the cycle closes back on the first,
  // proving advance-all plus wrap-to-first in one pass.
  for (let i = 0; i < starts.length; i += 1) {
    const top = await scrollTop(page);
    const cur = await rulerValueNow(page);
    await nextBtn.click();
    await expectJumpedTo(nextStartAfter(starts, cur), top);
  }
  await expect.poll(async () => rulerValueNow(page), { timeout: 10000 }).toBe(first);

  // Prev from the first wraps to the last, then walks back down.
  const last = starts[starts.length - 1] ?? 0;
  {
    const top = await scrollTop(page);
    await prevBtn.click();
    await expectJumpedTo(last, top);
  }
  {
    const secondLast = starts[starts.length - 2] ?? 0;
    const top = await scrollTop(page);
    await prevBtn.click();
    await expectJumpedTo(secondLast, top);
  }

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hunk-nav-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hunk-nav-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});

test("hunk nav actions disable for a non-text kind", async ({ page }) => {
  // Folder sessions render no ruler rows, so both actions must disable.
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill("/tmp/hunk14-left");
  await page.getByTestId("session/right-path").fill("/tmp/hunk14-right");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();
  await expect(page.getByTestId("toolbar/action-next-hunk")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-prev-hunk")).toBeDisabled();
});
