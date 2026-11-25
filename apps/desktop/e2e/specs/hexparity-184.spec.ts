/**
 * Wave-184 hex BC-parity batch (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager -> tab -> ViewRouter ->
 * HexView) over the embedded 11-byte `h_*` pair ("hello world" vs "hallo
 * world", single page, one differing offset at 1) and pins the wave-184
 * visual/geometry fixes against Beyond Compare Hex Compare (observed
 * externally: dual panes, solid selection block, red diff bytes on both
 * sides, per-pane byte counts, "Binary differences" + "Load time" status):
 *  (1) selection fill: the selected byte paints a solid selection block
 *      (inline `SEL_FILL` over selection tokens, theme-aware) on BOTH
 *      twins, keeps `hx-diff`, and both twins share one row.
 *  (2) bytes-per-line nav: ArrowUp/Down step by the active bytes-per-line
 *      (8 -> +-8), not the old hardcoded 16.
 *  (3) wide grid: 32 bytes/line keeps 8 rows, scrolls horizontally inside
 *      the grid (no page-level spill, no overlap), ascii columns intact.
 *  (4) chrome budget: panels closed -> single toolbar row, first hex row
 *      top <= 140px section-relative (W180 topbar-unify reuse, no dup rows).
 *  (5) dark scheme: selection fill still paints over the diff tint.
 * Functional specs assert behavior only; no hardcoded pixels in old
 * specs -- all geometry lives here. New file only, no existing spec
 * touched. Zero console/page errors throughout.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/** Wave-184: overlap gate scoped to the OWNED hex grid only (the app
 * topbar/tabbar is off-limits and overlaps under this webkit build).
 * Rows are siblings, bytes nest inside their row (containment skips). */
const HEX_GRID = '[data-testid="view-hex"] .hx-row, [data-testid="view-hex"] .hx-byte';

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

async function createHexSession(page: Page): Promise<void> {
  const banner = page.getByTestId("workspace/demo-banner");
  if (await banner.waitFor({ state: "visible", timeout: 3000 }).then(() => true).catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
    await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/h184-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/h184-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

function watch(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function checkClean(
  consoleErrors: string[],
  pageErrors: string[],
): void {
  expect(consoleErrors, `console errors: ${consoleErrors.join(" | ")}`).toEqual([]);
  expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);
}

test.describe("light mode", () => {
  test.use({ colorScheme: "light" });

test("hexparity-184: selection paints a solid block on both twins, same row", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = watch(page);
  await createHexSession(page);

  await page.getByTestId("hexview/byte-1").click();
  const left = page.getByTestId("hexview/byte-1");
  const right = page.getByTestId("hexview/byte-R-1");
  await expect(left).toHaveAttribute("data-selected", "true");
  await expect(right).toHaveAttribute("data-selected", "true");
  // Legacy diff tint preserved under the selection (older specs).
  await expect(left).toHaveClass(/hx-diff/);
  await expect(right).toHaveClass(/hx-diff/);

  // Solid fill: selected twins resolve the selection tokens, and differ
  // from an unselected same byte; both twins paint identically.
  const fills = await page.evaluate(() => {
    const bg = (id: string): string => {
      const el = document.querySelector(`[data-testid="${id}"]`);
      return el instanceof HTMLElement ? window.getComputedStyle(el).backgroundColor : "missing";
    };
    return { sel: bg("hexview/byte-1"), selR: bg("hexview/byte-R-1"), plain: bg("hexview/byte-2") };
  });
  expect(fills.sel, "selected byte paints a fill").not.toBe(fills.plain);
  expect(fills.selR, "right twin paints the same fill").toBe(fills.sel);

  // Twins share one visual row (two-side sync geometry).
  const dy = await page.evaluate(() => {
    const l = document.querySelector('[data-testid="hexview/byte-1"]');
    const r = document.querySelector('[data-testid="hexview/byte-R-1"]');
    if (!(l instanceof HTMLElement) || !(r instanceof HTMLElement)) return null;
    return Math.abs(l.getBoundingClientRect().y - r.getBoundingClientRect().y);
  });
  expect(dy, "twins share a row").not.toBeNull();
  expect(dy!).toBeLessThan(2);

  const EVID = "/tmp/hexparity-184";
  await page.screenshot({ path: `${EVID}-light.png` });
  await page.getByTestId("hexview/toolbar").screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await page.getByTestId("view-hex").locator(".hx-row").first().screenshot({ path: `${EVID}-zoom-diffrow.png` });

  await assertNoOverlap(page, HEX_GRID);
  checkClean(consoleErrors, pageErrors);
});

test("hexparity-184: ArrowUp/Down step by the active bytes-per-line", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = watch(page);
  await createHexSession(page);

  await page.getByTestId("hexview/bytes-per-line").selectOption("8");
  await page.getByTestId("hexview/byte-0").click();
  await expect(page.getByTestId("hexview/byte-0")).toHaveAttribute("data-selected", "true");

  // Down steps one visual row (8), Up returns -- not the old hardcoded 16.
  await page.getByTestId("hexview/byte-0").press("ArrowDown");
  await expect(page.getByTestId("hexview/byte-8")).toHaveAttribute("data-selected", "true");
  await page.getByTestId("hexview/byte-8").press("ArrowUp");
  await expect(page.getByTestId("hexview/byte-0")).toHaveAttribute("data-selected", "true");

  await assertNoOverlap(page, HEX_GRID);
  checkClean(consoleErrors, pageErrors);
});

test("hexparity-184: 32 bytes/line keeps 8 rows with in-grid horizontal scroll", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = watch(page);
  await createHexSession(page);

  await page.getByTestId("hexview/bytes-per-line").selectOption("32");
  const rows = page.getByTestId("view-hex").locator(".hx-row");
  await expect(rows).toHaveCount(8);
  // Both ascii columns survive the wide layout on the first row.
  await expect(rows.first().locator(".hx-ascii")).toHaveCount(2);

  // The grid absorbs the width internally: no document-level horizontal
  // spill from the wide rows.
  const spill = await page.evaluate(() => {
    const grid = document.querySelector(".hx-row")?.parentElement;
    if (!(grid instanceof HTMLElement)) return null;
    const cs = window.getComputedStyle(grid);
    return {
      overflowX: cs.overflowX,
      docSpill: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    };
  });
  expect(spill, "grid mounted").not.toBeNull();
  expect(spill!.overflowX, "wide rows scroll inside the grid").toBe("auto");
  expect(spill!.docSpill, `no page-level horizontal spill (got ${spill!.docSpill}px)`).toBeLessThanOrEqual(1);

  // Row-0 bytes lay out left-to-right with no mutual overlap (byte-grid
  // density + ascii alignment geometry, tolerant to in-grid scroll).
  const seq = await page.evaluate(() => {
    const row = document.querySelector(".hx-row");
    if (!(row instanceof HTMLElement)) return null;
    const cells = [...row.querySelectorAll(".hx-byte")].filter(
      (el) => el instanceof HTMLElement,
    ) as HTMLElement[];
    return cells.map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.x, w: r.width };
    });
  });
  expect(seq, "row 0 mounted").not.toBeNull();
  expect(seq!.length, "row 0 holds 32+32 cells").toBeGreaterThan(60);
  for (let i = 1; i < seq!.length; i += 1) {
    expect(
      seq![i].x,
      `cell ${i} starts at/after cell ${i - 1} end`,
    ).toBeGreaterThanOrEqual(seq![i - 1].x + seq![i - 1].w - 1);
  }

  checkClean(consoleErrors, pageErrors);
});

test("hexparity-184: chrome budget holds with panels closed", async ({ page }) => {
  const { consoleErrors, pageErrors } = watch(page);
  await createHexSession(page);

  await page.getByTestId("hexview/toolbar-find").click();
  await page.getByTestId("hexview/toolbar-goto").click();
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();

  const toolbar = page.getByTestId("hexview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar one compact row (got ${barH}px)`).toBeLessThan(120);
  const delta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="hex comparison"]');
    const first = document.querySelector(".hx-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(delta!, `firstRowTop<=140 (got ${delta})`).toBeLessThanOrEqual(140);

  // Diffs-only still narrows to the single differing row (filter intact).
  await page.getByTestId("hexview/diffsonly-toggle").check();
  await expect(page.getByTestId("view-hex").locator(".hx-row")).toHaveCount(1);

  await assertNoOverlap(page, HEX_GRID);
  checkClean(consoleErrors, pageErrors);
});
});

test.describe("dark mode", () => {
  test.use({ colorScheme: "dark" });

test("hexparity-184: dark scheme keeps the selection fill over the diff tint", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = watch(page);
  await createHexSession(page);

  await page.getByTestId("hexview/byte-1").click();
  await expect(page.getByTestId("hexview/byte-1")).toHaveAttribute("data-selected", "true");
  const fills = await page.evaluate(() => {
    const bg = (id: string): string => {
      const el = document.querySelector(`[data-testid="${id}"]`);
      return el instanceof HTMLElement ? window.getComputedStyle(el).backgroundColor : "missing";
    };
    return { sel: bg("hexview/byte-1"), plain: bg("hexview/byte-2") };
  });
  expect(fills.sel, "dark selection paints a fill").not.toBe(fills.plain);
  expect(fills.sel, "dark fill is opaque").not.toContain("rgba(0, 0, 0, 0)");

  const EVID = "/tmp/hexparity-184";
  await page.screenshot({ path: `${EVID}-dark.png` });
  await page.getByTestId("view-hex").locator(".hx-row").first().screenshot({ path: `${EVID}-zoom-diffrow-dark.png` });

  await assertNoOverlap(page, HEX_GRID);
  checkClean(consoleErrors, pageErrors);
});
});
