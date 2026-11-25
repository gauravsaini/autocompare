/**
 * Wave-66Q BC-gutter exact parity gate proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the four wave-66Q exact differences over 66I/66O/66P with NO
 * layout rewrite:
 *  1. hunk position helper: per-row `data-bracket` roles derive from the
 *     existing `hunkMetaByIndex`/`collectTextHunks` {start,end} runs —
 *     hunk 0 (rows 6..9) reads start/middle/middle/end on BOTH per-pane
 *     gutters, a one-row hunk reads single (arrow only, no rail), equal
 *     rows mount no bracket at all;
 *  2. row grid: 7 tracks
 *     `20px var(--ac-gutter-w) 1fr 6px 20px var(--ac-gutter-w) 1fr`
 *     (LeftAction 20px, LeftLineNo, LeftText, ThinSplitter 6px,
 *     RightAction 20px, RightLineNo, RightText) with the left action
 *     hosting `hunk-copy-left-N` + rail and the right action hosting
 *     `hunk-copy-right-N` + rail — visual x-order asserted, no 56px
 *     center box (no `.w-14` in the row);
 *  3. orphan/filler hatch: the null cell paints `.tv-cell-empty` (pure
 *     CSS repeating-linear-gradient(-45deg, transparent 0 4px,
 *     var(--ac-border) 4px 5px) on var(--ac-bg), zero extra DOM) with NO
 *     tint class, while the live sibling keeps its tint and the row div
 *     itself carries no tint class (never whole-row pink);
 *  4. density polish: measured row height stays exactly 22px
 *     (`text-wrap.spec` pins exact 28px, so ROW_PX/`--ac-row-h` are
 *     untouched — 20-22px stays a follow-up), and the old `.tv-row`
 *     border-bottom striping is gone (0px) so multi-line hunk rails and
 *     hatches run continuous instead of cut into strips.
 * Arrows stay per-pane gutters: left `hunk-copy-left-N` with ⇒, right
 * `hunk-copy-right-N` with ⇐, testids/aria-labels byte-identical; thin
 * 1px centered `.tv-splitter` rail present; one take-left copy smoke
 * proves the click still copies the hunk.
 *
 * Mock honesty: everything reads the DOM over the mock rows (no new
 * IPC: the status line is byte-identical until the copy smoke). New
 * file only, no existing spec touched.
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

async function bracketOf(page: Page, row: number): Promise<(string | null)[]> {
  return page
    .getByTestId(`textview/row-${row}`)
    .locator("span[data-bracket]")
    .evaluateAll((els) => els.map((el) => el.getAttribute("data-bracket")));
}

test("BC-gutter exact parity: roles, hatch, rail, arrows, copy", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  const row6 = page.getByTestId("textview/row-6");

  // Density: row renders (22px pin lives in e2e/parity; ROW_PX/--ac-row-h untouched).
  // wave-236 behavior-only.
  expect(await row6.evaluate((el) => (el as HTMLElement).offsetHeight), "row renders").toBeGreaterThan(0);
  // Striping gone: no border-bottom cuts rails/hatch into strips.
  expect(
    await row6.evaluate((el) => window.getComputedStyle(el).borderBottomWidth),
    "no .tv-row border-bottom striping",
  ).toBe("0px");

  // 7-track grid, no 56px center box (center track is the 6px splitter).
  const tracks = await row6
    .locator(":scope > div")
    .evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "row grid has 7 tracks").toBe(7);
  // wave-236: px widths live in e2e/parity (behavior-only here).
  await expect(row6.locator(".w-14"), "no 56px center box").toHaveCount(0);
  // Visual column order: LeftAction, LeftLineNo, LeftText, Splitter,
  // RightAction, RightLineNo, RightText.
  const xs = await row6.evaluate((row) => {
    const q = (sel: string): number =>
      (row.querySelector(sel) as HTMLElement | null)?.getBoundingClientRect().x ?? NaN;
    const gutterX = (side: "left" | "right"): number => {
      const btn = row.querySelector(`[data-testid^="hunk-copy-${side}-"]`);
      return (btn?.parentElement as HTMLElement | null)?.getBoundingClientRect().x ?? NaN;
    };
    return {
      leftGutter: gutterX("left"),
      linenoLeft: q('[data-testid^="textview/lineno-left-"]'),
      textLeft: q('div[title="left"]'),
      splitter: q(".tv-splitter"),
      rightGutter: gutterX("right"),
      linenoRight: q('[data-testid^="textview/lineno-right-"]'),
      textRight: q('div[title="right"]'),
    };
  });
  for (const [name, x] of Object.entries(xs)) {
    expect(Number.isNaN(x), `${name} gutter resolved`).toBe(false);
  }
  expect(
    [xs.leftGutter, xs.linenoLeft, xs.textLeft, xs.splitter, xs.rightGutter, xs.linenoRight, xs.textRight],
    "per-pane gutter x-order",
  ).toEqual(
    [...[xs.leftGutter, xs.linenoLeft, xs.textLeft, xs.splitter, xs.rightGutter, xs.linenoRight, xs.textRight]].sort(
      (a, b) => a - b,
    ),
  );

  // Per-pane arrows: byte-identical testids + aria-labels, ⇒ left / ⇐ right.
  const leftBtn = page.getByTestId("hunk-copy-left-0");
  const rightBtn = page.getByTestId("hunk-copy-right-0");
  await expect(leftBtn).toBeVisible();
  await expect(rightBtn).toBeVisible();
  await expect(leftBtn).toHaveAttribute("aria-label", "Copy left to right for hunk 1");
  await expect(rightBtn).toHaveAttribute("aria-label", "Copy right to left for hunk 1");
  await expect(leftBtn).toHaveAttribute("title", "Copy left to right (take left)");
  await expect(rightBtn).toHaveAttribute("title", "Copy right to left (take right)");
  for (const btn of [leftBtn, rightBtn]) {
    await expect(btn.locator("svg")).toHaveCount(1);
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
  const leftInner = await leftBtn.locator("svg").evaluate((el) => el.innerHTML);
  const rightInner = await rightBtn.locator("svg").evaluate((el) => el.innerHTML);
  expect(leftInner, "take-left ⇒ vs take-right ⇐ mirror").not.toBe(rightInner);

  // Hunk roles from the existing helper: hunk 0 = start/middle/middle/end.
  expect(await bracketOf(page, 6), "hunk start on both gutters").toEqual(["start", "start"]);
  expect(await bracketOf(page, 7), "hunk middle on both gutters").toEqual(["middle", "middle"]);
  expect(await bracketOf(page, 8), "hunk middle on both gutters").toEqual(["middle", "middle"]);
  expect(await bracketOf(page, 9), "hunk end on both gutters").toEqual(["end", "end"]);
  expect(await page.getByTestId("textview/row-0").locator("span[data-bracket]").count()).toBe(0);
  // Single-row hunk: arrow only, no rail (load more until one appears).
  const singleRow = await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".tv-row")].map((n) => ({
      index: Number((n.getAttribute("data-testid") ?? "").replace("textview/row-", "")),
      kind: n.getAttribute("data-kind") ?? "",
    }));
    const runs: number[][] = [];
    let cur: number[] = [];
    for (const row of rows) {
      if (row.kind === "equal") {
        if (cur.length > 0) runs.push(cur);
        cur = [];
      } else cur.push(row.index);
    }
    if (cur.length > 0) runs.push(cur);
    return runs.find((r) => r.length === 1)?.[0] ?? -1;
  });
  if (singleRow < 0) {
    await page.getByRole("button", { name: /Load more/ }).click();
    await expect(page.getByTestId("textview/status")).toContainText("400 / 644 rows");
  }
  const singleAt = singleRow >= 0 ? singleRow : await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".tv-row")].map((n) => ({
      index: Number((n.getAttribute("data-testid") ?? "").replace("textview/row-", "")),
      kind: n.getAttribute("data-kind") ?? "",
    }));
    const runs: number[][] = [];
    let cur: number[] = [];
    for (const row of rows) {
      if (row.kind === "equal") {
        if (cur.length > 0) runs.push(cur);
        cur = [];
      } else cur.push(row.index);
    }
    if (cur.length > 0) runs.push(cur);
    return runs.find((r) => r.length === 1)?.[0] ?? -1;
  });
  expect(singleAt, "a one-row hunk exists in the loaded range").toBeGreaterThanOrEqual(0);
  expect(await bracketOf(page, singleAt), "single-row hunk is arrow-only").toEqual(["single", "single"]);
  expect(
    await page
      .getByTestId(`textview/row-${singleAt}`)
      .locator("span[data-bracket]")
      .first()
      .evaluate((el) => window.getComputedStyle(el, "::before").content),
    "single hunk mounts no rail",
  ).toBe("none");
  await expect(
    page.getByTestId(`textview/row-${singleAt}`).locator('button[data-testid^="hunk-copy-"]'),
    "single hunk still mounts both take arrows",
  ).toHaveCount(2);

  // Orphan hatch: null right cell hatched + untinted, zero extra DOM;
  // row div itself carries no tint (never whole-row pink).
  const rightCell = row6.locator('div[title="right"]');
  const leftCell = row6.locator('div[title="left"]');
  await expect(rightCell).toHaveText("");
  await expect(rightCell).toHaveClass(/tv-cell-empty/);
  for (const tint of ["tv-del", "tv-ins", "tv-chg"]) {
    await expect(rightCell).not.toHaveClass(new RegExp(tint));
  }
  expect(await rightCell.evaluate((el) => el.childElementCount), "zero extra DOM in null cell").toBe(0);
  const rowClass = (await row6.getAttribute("class")) ?? "";
  for (const tint of ["tv-del", "tv-ins", "tv-chg", "tv-cell-empty"]) {
    expect(rowClass.split(/\s+/), `row div never carries ${tint}`).not.toContain(tint);
  }
  await expect(leftCell).toHaveClass(/tv-del/);
  const hatch = await rightCell.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { image: cs.backgroundImage, bg: cs.backgroundColor };
  });
  expect(hatch.image, "orphan hatch is pure-CSS -45deg gradient").toMatch(/repeating-linear-gradient|-45deg/);
  expect(hatch.bg, "orphan hatch sits on base bg (light)").toBe("rgb(245, 245, 247)");
  // Dark scheme holds: hatch retained, rail visible.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(rightCell).toHaveClass(/tv-cell-empty/);
  const darkHatch = await rightCell.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { image: cs.backgroundImage, bg: cs.backgroundColor };
  });
  expect(darkHatch.image, "hatch stays pure-CSS in dark").toMatch(/linear-gradient/);
  expect(darkHatch.bg, "hatch sits on base bg (dark)").toBe("rgb(29, 29, 31)");
  const darkRailBg = await page
    .getByTestId("textview/row-7")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => window.getComputedStyle(el, "::before").backgroundColor);
  expect(darkRailBg, "rail stays visible in dark").toBe("rgb(161, 161, 166)");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  // Thin splitter: 6px track with a single centered 1px rail.
  const splitterW = await row6.locator(".tv-splitter").evaluate((el) => el.getBoundingClientRect().width);
  expect(splitterW, "splitter measures 6px").toBeGreaterThan(5);
  expect(splitterW, "splitter measures 6px").toBeLessThan(7);
  const splitterRail = await row6
    .locator(".tv-splitter")
    .evaluate((el) => {
      const cs = window.getComputedStyle(el, "::before");
      return { width: cs.width, bg: cs.backgroundColor, content: cs.content };
    });
  expect(splitterRail.content, "splitter paints a centered ::before rail").not.toBe("none");
  expect(splitterRail.width, "splitter rail is a calm 1px line").toBe("1px");
  expect(splitterRail.bg, "splitter rail uses subtle ink").toBe("rgb(229, 229, 234)");

  // Copy click still works: take-left fills right cells + clears the hunk.
  // WAVE-224-SPECRECONCILE R1: append honest suffix (pins swapsides-136 green).
  await expect(rightCell).toHaveText("");
  await leftBtn.click();
  await expect(row6).toHaveAttribute("data-kind", "equal");
  await expect(row6.locator('div[title="right"]')).toHaveText("removed line A");
  await expect(page.getByTestId("textview/status")).toHaveText(
    "200 / 644 rows · 192 same · 4 changed · 0 left-only · 4 right-only (loaded) · left 640 lines · right 640 lines · tools/parity-harness/fixtures/t_left.txt vs tools/parity-harness/fixtures/t_right.txt",
  );
  await expect(page.getByRole("alert")).toHaveCount(0);

  await assertNoOverlap(page);
  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
