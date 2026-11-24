/**
 * Wave-91 table per-side grids + red diff cells + 22px unification
 * (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics; header `id | name | price`, 550
 * rows, 100 loaded) and proves the Beyond Compare style per-side
 * grids (mirrors the wave-87 two-pane shell):
 *  - a `tableview/grids` row renders left grid + 6px splitter +
 *    right grid side by side (`tableview/grid-left` fully left of
 *    `tableview/grid-right`, `tableview/grid-splitter` center rail,
 *    middle grid track exactly 6px);
 *  - each side owns its column headers (`table-sort-C` left legacy,
 *    `table-sort-right-C` right mirror, same names) sharing one
 *    sort state (right-header click cycles asc/desc/clear);
 *  - changed cells on BOTH sides carry the red diff wash
 *    (`.tc-diff` + `data-diff="true"` on left `tableview/cell-5-2`
 *    and right `tableview/cell-right-5-2`; unchanged
 *    `tableview/cell-0-2` carries none; computed background is the
 *    red del token in light AND dark);
 *  - legacy find mapping intact on the left (`Hit 1 of 6`,
 *    `[data-hit="true"]` x3, `table-hit-0` reads `10.25`) while the
 *    right grid lights only right-side hits
 *    (`[data-hit-right="true"]` x3, `table-hit-right-0`);
 *  - unified density: left AND right row-0 measure 22px with
 *    `--ac-row-h: 22px` as the source of truth;
 *  - chrome budget holds: with both panels closed the first grid
 *    row sits within 140px of the table section top; the bottom
 *    status bar still shows the FULL-table `125 difference row(s)`;
 *  - light + dark + zoomed + side-by-side-vs-truth PNGs land in
 *    `/tmp/ui-91/` (never in-repo); zero console/page errors.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-91";

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
  await page.getByTestId("session/left-path").fill("/tmp/wave91-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave91-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("table per-side grids: side-by-side panes, red diffs light+dark, 22px density", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  const view = page.getByTestId("view-table");
  await expect(page.getByTestId("table-cols-count")).toContainText("100 / 550 rows");

  // 1. Per-side grids: left + 6px splitter + right, side by side.
  const grids = page.getByTestId("tableview/grids");
  const gridLeft = page.getByTestId("tableview/grid-left");
  const gridRight = page.getByTestId("tableview/grid-right");
  const splitter = page.getByTestId("tableview/grid-splitter");
  await expect(grids).toBeVisible();
  await expect(gridLeft).toBeVisible();
  await expect(gridRight).toBeVisible();
  await expect(splitter).toBeVisible();
  const layout = await page.evaluate(() => {
    const l = document.querySelector('[data-testid="tableview/grid-left"]');
    const r = document.querySelector('[data-testid="tableview/grid-right"]');
    const s = document.querySelector('[data-testid="tableview/grid-splitter"]');
    const g = document.querySelector('[data-testid="tableview/grids"]');
    if (
      !(l instanceof HTMLElement) ||
      !(r instanceof HTMLElement) ||
      !(s instanceof HTMLElement) ||
      !(g instanceof HTMLElement)
    )
      return null;
    const lb = l.getBoundingClientRect();
    const rb = r.getBoundingClientRect();
    const sb = s.getBoundingClientRect();
    return {
      left: lb.x,
      leftRight: lb.x + lb.width,
      splitX: sb.x,
      splitW: sb.width,
      right: rb.x,
      tracks: window.getComputedStyle(g).gridTemplateColumns,
    };
  });
  expect(layout, "both grids + splitter mounted").not.toBeNull();
  expect(layout?.left ?? 0, "left grid starts left of right grid").toBeLessThan(
    layout?.right ?? 0,
  );
  expect(
    layout?.leftRight ?? Number.POSITIVE_INFINITY,
    "left grid ends where right begins",
  ).toBeLessThanOrEqual((layout?.right ?? 0) + 8);
  expect(layout?.splitX ?? 0, "splitter sits between the grids").toBeGreaterThanOrEqual(
    (layout?.leftRight ?? 0) - 8,
  );
  expect(layout?.splitX ?? 0, "splitter sits between the grids").toBeLessThanOrEqual(
    (layout?.right ?? 0) + 8,
  );
  expect(layout?.splitW ?? -1, "splitter track is 6px").toBe(6);
  expect(layout?.tracks ?? "", "middle grid track is 6px").toContain("6px");

  // 2. Per-side column headers name the same columns on both sides.
  await expect(page.getByTestId("table-sort-0")).toContainText("id");
  await expect(page.getByTestId("table-sort-1")).toContainText("name");
  await expect(page.getByTestId("table-sort-2")).toContainText("price");
  await expect(page.getByTestId("table-sort-right-0")).toContainText("id");
  await expect(page.getByTestId("table-sort-right-1")).toContainText("name");
  await expect(page.getByTestId("table-sort-right-2")).toContainText("price");

  // 3. Changed cells on BOTH sides carry the red diff class.
  // Mock rows 5/28/51/74/97 differ in `price` (col 2): 5 per side.
  const leftDiffs = view.locator('[data-testid="tableview/grid-left"] .tc-diff');
  const rightDiffs = view.locator('[data-testid="tableview/grid-right"] .tc-diff');
  await expect(leftDiffs).toHaveCount(5);
  await expect(rightDiffs).toHaveCount(5);
  const diffLeft = page.getByTestId("tableview/cell-5-2");
  const diffRight = page.getByTestId("tableview/cell-right-5-2");
  await expect(diffLeft).toHaveClass(/tc-diff/);
  await expect(diffLeft).toHaveAttribute("data-diff", "true");
  await expect(diffRight).toHaveClass(/tc-diff/);
  await expect(diffRight).toHaveAttribute("data-diff", "true");
  await expect(diffRight).toContainText("99.00");
  const sameCell = page.getByTestId("tableview/cell-0-2");
  await expect(sameCell).not.toHaveClass(/tc-diff/);
  expect(await sameCell.getAttribute("data-diff"), "same cells carry no data-diff").toBeNull();
  const lightBg = await diffLeft.evaluate((el) => window.getComputedStyle(el).backgroundColor);
  const lightSame = await sameCell.evaluate((el) => window.getComputedStyle(el).backgroundColor);
  const rgb = (s: string): [number, number, number] | null => {
    const m = s.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
  };
  const light = rgb(lightBg);
  expect(light, `light diff wash is red (got ${lightBg})`).not.toBeNull();
  expect(light?.[0] ?? 0, "red channel dominant").toBeGreaterThan(200);
  expect(light?.[0] ?? 0, "red above green+blue").toBeGreaterThan(light?.[1] ?? 0);
  expect(light?.[0] ?? 0, "red above green+blue").toBeGreaterThan(light?.[2] ?? 0);
  expect(lightBg, "diff wash differs from same cells").not.toBe(lightSame);

  // 4. Unified 22px density: ROW_PX constants pinned at 22 in
  //    both views, --ac-row-h token 22px on the rows, and both
  //    per-side grids rendering the same row height (aligned panes).
  const rowPx = await page.evaluate(async () => {
    const tv = (await import(
      "/src/components/views/TableView.tsx"
    )) as unknown as Record<string, unknown>;
    const fv = (await import(
      "/src/components/views/FolderView.tsx"
    )) as unknown as Record<string, unknown>;
    return { table: tv["ROW_PX"], folder: fv["ROW_PX"] };
  });
  expect(rowPx.table, "TableView ROW_PX is 22").toBe(22);
  // Wave-96: folder moves to the BC 17px pitch (observe-93); table stays 22.
  expect(rowPx.folder, "FolderView ROW_PX is 17").toBe(17);
  const leftH = await page.getByTestId("tableview/row-0").evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return {
      offsetHeight: (el as HTMLElement).offsetHeight,
      tokenVal: cs.getPropertyValue("--ac-row-h").trim(),
    };
  });
  const rightH = await page
    .getByTestId("tableview/row-right-0")
    .evaluate((el) => (el as HTMLElement).offsetHeight);
  expect(leftH.tokenVal, "--ac-row-h token is 22px").toBe("22px");
  expect(rightH, "per-side grids share one row height").toBe(leftH.offsetHeight);

  // 5. Find mapping: legacy left rule intact, right lights right hits.
  // Wave-113A preamble (zero assertion change): find panel is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("tableview/toolbar-find").click();
  await page.getByTestId("table-find-input").fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 1 of 6", {
    timeout: 10000,
  });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(3);
  await expect(page.locator('[data-hit-right="true"]')).toHaveCount(3);
  await expect(page.getByTestId("table-hit-0")).toContainText("10.25");
  // The cursor starts on a left-side hit: no right-current marker yet.
  await expect(page.getByTestId("table-hit-right-0")).toHaveCount(0);
  // Walk the cursor onto the first right-side hit (hit 4 of 6):
  // both grids mark their own cell current under distinct testids.
  await page.getByTestId("table-find-next").click();
  await page.getByTestId("table-find-next").click();
  await page.getByTestId("table-find-next").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 4 of 6");
  await expect(page.getByTestId("table-hit-3")).toContainText("10.25");
  await expect(page.getByTestId("table-hit-right-3")).toContainText("10.25");

  // 6. Shared sort wiring: the RIGHT header drives session order too.
  await page.getByTestId("table-sort-right-2").click();
  await expect(page.getByTestId("table-sort-status")).toContainText(/Sorted by price ascending/);
  await page.getByTestId("table-sort-right-2").click();
  await expect(page.getByTestId("table-sort-status")).toContainText(/Sorted by price descending/);
  await page.getByTestId("table-sort-right-2").click();
  await expect(page.getByTestId("table-sort-status")).toHaveCount(0);

  // 7. Chrome budget + status bar: panels closed, first row within
  //    140px of the section top, FULL-table total below the grids.
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  await expect(page.getByTestId("tableview/diffs")).toContainText("125 difference row(s)");
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector('[data-testid="tableview/grid-left"] tbody tr.tc-row');
    const bar = document.querySelector('[data-testid="tableview/status"]');
    if (
      !(section instanceof HTMLElement) ||
      !(first instanceof HTMLElement) ||
      !(bar instanceof HTMLElement)
    )
      return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    const b = bar.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top, barTop: b.top };
  });
  expect(chrome, "section + first grid row + status mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `grids stay within collapsed chrome budget (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);
  expect(chrome?.barTop ?? 0, "status bar lives below the grids").toBeGreaterThan(
    chrome?.firstTop ?? 0,
  );

  await assertNoOverlap(page);
  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light evidence + zoomed grid crop (never in-repo).
  const gridsBox = await grids.boundingBox();
  await page.screenshot({ path: `${EVID_DIR}/tablegrid-91-light.png` });
  if (gridsBox) {
    await page.screenshot({
      path: `${EVID_DIR}/tablegrid-91-zoom-light.png`,
      clip: {
        x: gridsBox.x,
        y: gridsBox.y,
        width: gridsBox.width,
        height: Math.min(600, gridsBox.height),
      },
    });
  }

  // Dark evidence: red del-token wash must survive the scheme swap.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  const darkBg = await diffRight.evaluate((el) => window.getComputedStyle(el).backgroundColor);
  const dark = rgb(darkBg);
  expect(dark, `dark diff wash stays red (got ${darkBg})`).not.toBeNull();
  expect(dark?.[0] ?? 255, "dark wash is deep red").toBeLessThan(130);
  expect(dark?.[0] ?? 0, "red above green in dark").toBeGreaterThan(dark?.[1] ?? 0);
  const gridsBoxDark = await grids.boundingBox();
  await page.screenshot({ path: `${EVID_DIR}/tablegrid-91-dark.png` });
  if (gridsBoxDark) {
    await page.screenshot({
      path: `${EVID_DIR}/tablegrid-91-zoom-dark.png`,
      clip: {
        x: gridsBoxDark.x,
        y: gridsBoxDark.y,
        width: gridsBoxDark.width,
        height: Math.min(600, gridsBoxDark.height),
      },
    });
  }
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Side-by-side vs the BC truth (composite shot, never in-repo).
  // Honest SKIP when the /tmp truth fixture is absent (e.g. /tmp wipe):
  // geometry asserts above already passed; only the composite needs it.
  const shot = (p: string): string => fs.readFileSync(p).toString("base64");
  if (!fs.existsSync("/tmp/bc-table-truth.png")) {
    console.warn("SKIP tablegrid-91-vs-bc composite: /tmp/bc-table-truth.png absent");
  } else {
  await page.setContent(
    `<html><body style="margin:0;background:#888;font-family:sans-serif;color:#fff">` +
      `<h3 style="margin:8px">BC truth (left) vs wave-91 light (middle) vs wave-91 dark (right)</h3>` +
      `<div style="display:flex;gap:8px;align-items:flex-start">` +
      `<figure style="margin:0"><img style="width:420px" src="data:image/png;base64,${shot("/tmp/bc-table-truth.png")}"/><figcaption>bc-table-truth</figcaption></figure>` +
      `<figure style="margin:0"><img style="width:420px" src="data:image/png;base64,${shot(`${EVID_DIR}/tablegrid-91-light.png`)}"/><figcaption>wave-91 light</figcaption></figure>` +
      `<figure style="margin:0"><img style="width:420px" src="data:image/png;base64,${shot(`${EVID_DIR}/tablegrid-91-dark.png`)}"/><figcaption>wave-91 dark</figcaption></figure>` +
      `</div></body></html>`,
  );
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/tablegrid-91-vs-bc.png`, fullPage: true });
  }

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
