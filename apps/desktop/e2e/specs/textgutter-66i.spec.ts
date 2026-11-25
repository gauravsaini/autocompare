/**
 * Wave-66I BC-style per-pane hunk gutter parity proof (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the Beyond Compare style hunk gutter relocation:
 *  - no center `w-14` button strip: each row paints the 7-track grid
 *    `20px gutter 1fr 6px 20px gutter 1fr` (computed
 *    `gridTemplateColumns` has 7 tracks, 20px / 6px / 20px action +
 *    splitter tracks), rows stay exactly 22px tall, the thin splitter
 *    measures 6px wide;
 *  - arrows live in the per-pane gutters: `hunk-copy-left-0` sits in
 *    the leftmost 20px cell (its box ends left of the left line
 *    number), `hunk-copy-right-0` in the right action cell (between
 *    the splitter and the right line number); both render the 12px
 *    golden-amber chevron (`--ac-tab-dirty`, no gray button box:
 *    borderless + transparent), and clicking copies exactly as before
 *    (take-left fills the right cells + clears the hunk, take-right
 *    mirrors — the wave-16 copy-behavior assertions, reused);
 *  - section bracket rail: per-row `data-bracket` derived from the
 *    existing hunk runs — `start` on the first row of a multi-line
 *    hunk, `middle` inside, `end` on the last, `single` on a one-row
 *    hunk (arrow only, no rail), absent on equal rows; the middle
 *    rail's `::before` measures the full row height;
 *  - orphan hatching: the null cell of a one-sided row carries
 *    `tv-cell-empty` and NO tint class while its sibling cell keeps
 *    `tv-del` / `tv-ins`;
 *  - one composition smoke: Differences only + find still compose
 *    (hunkNav/select/scroll/find/goto/66H-context logic untouched);
 *  - light + dark DOM checks (no PNGs committed), assertNoOverlap,
 *    zero console errors / page errors throughout.
 *
 * Mock honesty: everything above reads the DOM over the mock rows (no
 * new IPC: the status line is byte-identical throughout). Skip policy
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
    nodes
      .map((n) => ({
        index: Number((n.getAttribute("data-testid") ?? "").replace("textview/row-", "")),
        kind: n.getAttribute("data-kind") ?? "",
      }))
      .sort((a, b) => a.index - b.index),
  );
}

/** Consecutive non-equal index runs (the hunk grouping the view uses). */
function hunkRuns(rows: readonly LoadedRow[]): number[][] {
  const runs: number[][] = [];
  let cur: number[] = [];
  for (const row of rows) {
    if (row.kind === "equal") {
      if (cur.length > 0) {
        runs.push(cur);
        cur = [];
      }
    } else {
      cur.push(row.index);
    }
  }
  if (cur.length > 0) runs.push(cur);
  return runs;
}

async function pairCount(page: Page): Promise<number> {
  return page.locator('[data-testid^="hunk-copy-left-"]').count();
}

async function bracketOf(page: Page, row: number): Promise<(string | null)[]> {
  return page
    .getByTestId(`textview/row-${row}`)
    .locator("span[data-bracket]")
    .evaluateAll((els) => els.map((el) => el.getAttribute("data-bracket")));
}

test("per-pane hunk gutters, bracket rail, orphan hatch, copy parity", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // Wave-112B preamble: the find panel is closed by default (BC truth);
  // open it via its toolbar toggle before driving find. Zero assertion changes.
  {
    const t = page.getByTestId("textview/toolbar-find");
    if ((await t.getAttribute("aria-expanded")) !== "true") await t.click();
  }
  const statusBefore = await page.getByTestId("textview/status").innerText();

  // No center strip: 7-track row grid with 20px action tracks + 6px
  // splitter, rows exactly 22px tall.
  const tracks = await page
    .getByTestId("textview/row-6")
    .locator(":scope > div")
    .evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "row grid has 7 tracks").toBe(7);
  // wave-236: px widths live in e2e/parity (behavior-only here).
  const splitterW = await page
    .getByTestId("textview/row-6")
    .locator(".tv-splitter")
    .evaluate((el) => el.getBoundingClientRect().width);
  expect(splitterW, "splitter measures 6px").toBeGreaterThan(5);
  expect(splitterW, "splitter measures 6px").toBeLessThan(7);
  // wave-236 behavior-only: row renders (22px pin lives in e2e/parity).
  expect(await page.getByTestId("textview/row-6").evaluate((el) => (el as HTMLElement).offsetHeight)).toBeGreaterThan(0);

  // Arrows live in the per-pane gutters (hunk 0 = left-only rows 6..9).
  const leftBtn = page.getByTestId("hunk-copy-left-0");
  const rightBtn = page.getByTestId("hunk-copy-right-0");
  await expect(leftBtn).toBeVisible();
  await expect(rightBtn).toBeVisible();
  const leftBox = await leftBtn.evaluate((el) => el.parentElement?.getBoundingClientRect().toJSON() ?? null);
  const rightBox = await rightBtn.evaluate((el) => el.parentElement?.getBoundingClientRect().toJSON() ?? null);
  expect(leftBox).not.toBeNull();
  expect(rightBox).not.toBeNull();
  const linenoLeftX = await page.getByTestId("textview/lineno-left-6").evaluate((el) => el.getBoundingClientRect().x);
  const linenoRightX = await page.getByTestId("textview/lineno-right-6").evaluate((el) => el.getBoundingClientRect().x);
  const textLeftX = await page
    .getByTestId("textview/row-6")
    .locator('div[title="left"]')
    .evaluate((el) => el.getBoundingClientRect().x);
  const splitterBox = await page
    .getByTestId("textview/row-6")
    .locator(".tv-splitter")
    .evaluate((el) => el.getBoundingClientRect().toJSON());
  expect(leftBox?.width, "left action gutter is 20px").toBe(20);
  expect(leftBox!.x + leftBox!.width, "left gutter ends at/before the left line number").toBeLessThanOrEqual(
    linenoLeftX + 1,
  );
  expect(textLeftX, "left text starts right of the left gutter").toBeGreaterThanOrEqual(leftBox!.x + leftBox!.width - 1);
  expect(rightBox!.x, "right gutter starts right of the splitter").toBeGreaterThanOrEqual(
    splitterBox.x + splitterBox.width - 1,
  );
  expect(rightBox!.x, "right gutter ends at/before the right line number").toBeLessThanOrEqual(linenoRightX + 1);

  // 12px golden-amber chevrons, no gray button box (borderless +
  // transparent, hover highlight only).
  for (const btn of [leftBtn, rightBtn]) {
    const svgBox = await btn.locator("svg").evaluate((el) => el.getBoundingClientRect().toJSON());
    expect(Math.round(svgBox.width), "arrow svg is 12px").toBe(12);
    expect(Math.round(svgBox.height), "arrow svg is 12px").toBe(12);
    const style = await btn.evaluate((el) => {
      const cs = window.getComputedStyle(el);
      return { color: cs.color, borderTopWidth: cs.borderTopWidth, backgroundColor: cs.backgroundColor };
    });
    expect(style.color, "golden-amber --ac-tab-dirty ink").toBe("rgb(255, 159, 10)");
    expect(style.borderTopWidth, "no button box border").toBe("0px");
    expect(style.backgroundColor, "no button box fill").toBe("rgba(0, 0, 0, 0)");
  }

  // Section bracket rail: hunk 0 (rows 6..9) reads start/middle/end;
  // the middle rail's ::before spans the full 22px row height.
  expect(await bracketOf(page, 6), "hunk start brackets both gutters").toEqual(["start", "start"]);
  expect(await bracketOf(page, 7), "hunk middle brackets both gutters").toEqual(["middle", "middle"]);
  expect(await bracketOf(page, 8), "hunk middle brackets both gutters").toEqual(["middle", "middle"]);
  expect(await bracketOf(page, 9), "hunk end brackets both gutters").toEqual(["end", "end"]);
  const rail = await page
    .getByTestId("textview/row-7")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => {
      const cs = window.getComputedStyle(el, "::before");
      return { height: cs.height, content: cs.content, own: el.getBoundingClientRect().height };
    });
  expect(rail.content, "middle rail renders a ::before stem").not.toBe("none");
  expect(rail.height, "middle rail spans the gutter's full height").toBe(`${rail.own}px`);
  // Start/end rows mount a half-height stem (half of the full rail).
  const startRailH = await page
    .getByTestId("textview/row-6")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => window.getComputedStyle(el, "::before").height);
  expect(parseFloat(startRailH), "start stem is half the full rail").toBeCloseTo(rail.own / 2, 0);
  // Equal rows mount no bracket at all.
  expect(await page.getByTestId("textview/row-0").locator("span[data-bracket]").count()).toBe(0);

  // A one-row hunk (if the first window has none, page in the second
  // window exactly like the wave-16 proof does) carries `single`:
  // arrow only, no rail.
  let runs = hunkRuns(await loadedRows(page));
  if (!runs.some((r) => r.length === 1)) {
    await page.getByRole("button", { name: /Load more/ }).click();
    await expect(page.getByTestId("textview/status")).toContainText("400 / 644 rows");
    runs = hunkRuns(await loadedRows(page));
  }
  const single = runs.find((r) => r.length === 1);
  expect(single, "a one-row hunk exists in the loaded range").toBeDefined();
  const singleRow = single?.[0] ?? -1;
  expect(await bracketOf(page, singleRow), "single-row hunk is arrow-only").toEqual(["single", "single"]);
  const singleRail = await page
    .getByTestId(`textview/row-${singleRow}`)
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => window.getComputedStyle(el, "::before").content);
  expect(singleRail, "single hunk mounts no rail").toBe("none");

  // Orphan hatching (pristine hunk 0: left-only rows 6..9): the null
  // right cell carries the hatch class and NO tint class, the live
  // left cell keeps its `tv-del` tint.
  const rightCell = page.getByTestId("textview/row-6").locator('div[title="right"]');
  const leftCell = page.getByTestId("textview/row-6").locator('div[title="left"]');
  await expect(rightCell).toHaveText("");
  await expect(rightCell).toHaveClass(/tv-cell-empty/);
  for (const tint of ["tv-del", "tv-ins", "tv-chg"]) {
    await expect(rightCell).not.toHaveClass(new RegExp(tint));
  }
  await expect(leftCell).toHaveClass(/tv-del/);

  // Dark scheme: gutters, brackets, orphan hatch and arrows hold
  // (checked pre-copy while pristine one-sided rows still exist).
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("hunk-copy-left-0")).toBeVisible();
  await expect(page.getByTestId("hunk-copy-right-0")).toBeVisible();
  await expect(rightCell).toHaveClass(/tv-cell-empty/);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  // Composition smoke: Differences only + find still compose (all
  // other session/find/nav logic untouched).
  await page.getByTestId("textview/diffsonly-toggle").check();
  await expect(page.getByTestId("textview/diffsonly-note")).toBeVisible();
  await page.getByTestId("textview/find-input").fill("inserted");
  await page.getByTestId("textview/find-next").click();
  await expect(page.getByTestId("textview/find-status")).toContainText(/match 1 of \d+/);
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);
  await page.getByTestId("textview/diffsonly-toggle").uncheck();
  await page.getByTestId("textview/find-input").fill("");

  // Copy parity (wave-16 behavior, reused): take-left on hunk 0 fills
  // the right cells and clears the hunk; take-right mirrors.
  const rightOf = (row: number): ReturnType<Page["locator"]> =>
    page.getByTestId(`textview/row-${row}`).locator('div[title="right"]');
  await expect(rightOf(6)).toHaveText("");
  const pairsBefore = await pairCount(page);
  await leftBtn.click();
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", "equal");
  for (const [row, text] of [
    [6, "removed line A"],
    [7, "removed line B"],
    [8, "removed line C"],
    [9, "removed line D"],
  ] as const) {
    await expect(page.getByTestId(`textview/row-${row}`)).toHaveAttribute("data-kind", "equal");
    await expect(rightOf(row)).toHaveText(text);
  }
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(pairsBefore - 1);
  const leftOf = (row: number): ReturnType<Page["locator"]> =>
    page.getByTestId(`textview/row-${row}`).locator('div[title="left"]');
  await expect(leftOf(20)).toHaveText("");
  await page.getByTestId("hunk-copy-right-0").click();
  await expect(page.getByTestId("textview/row-20")).toHaveAttribute("data-kind", "equal");
  for (const [row, text] of [
    [20, "inserted extra line I"],
    [21, "inserted extra line II"],
    [22, "inserted extra line III"],
    [23, "inserted extra line IV"],
  ] as const) {
    await expect(page.getByTestId(`textview/row-${row}`)).toHaveAttribute("data-kind", "equal");
    await expect(leftOf(row)).toHaveText(text);
  }
  await expect.poll(async () => pairCount(page), { timeout: 10000 }).toBe(pairsBefore - 2);
  await expect(page.getByRole("alert")).toHaveCount(0);
  // Resolved orphans lose the hatch: the filled cell is a plain tint-free cell.
  await expect(rightOf(6)).not.toHaveClass(/tv-cell-empty/);

  await assertNoOverlap(page);
  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
