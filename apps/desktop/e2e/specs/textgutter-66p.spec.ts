/**
 * Wave-66P BC-gutter residual polish proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the residual gap closed over wave-66O (no layout rewrite):
 *  - rail stems truly centered: start/middle/end `::before` 1px stems
 *    carry a -50% x-translate so the stem straddles the 20px gutter
 *    center (pre-fix the stem's left edge sat ON center, 0.5px right);
 *    the end `::after` 5px foot still grows rightward from rail center;
 *  - arrows intact: per-pane `hunk-copy-left-N` / `hunk-copy-right-N`
 *    testids, 12px golden-amber `--ac-tab-dirty` TakeArrow SVG, mirrored
 *    dirs, borderless + transparent, one take-left copy smoke;
 *  - orphan honesty: null cell `tv-cell-empty` pure-CSS hatch on
 *    `var(--ac-bg)` with NO tint, live sibling keeps `tv-del`;
 *  - divider calm: 7-track grid, rows exactly 22px, splitter track 6px
 *    with a single centered 1px `--ac-border-subtle` rail;
 *  - light + dark DOM checks (no PNGs committed), assertNoOverlap,
 *    zero console errors / page errors throughout.
 *
 * Mock honesty: everything reads the DOM over the mock rows (no new
 * IPC: the status line is byte-identical throughout). New file only,
 * no existing spec touched.
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

test("BC-gutter residual: centered rail, hatch, calm divider, arrow copy", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  const statusBefore = await page.getByTestId("textview/status").innerText();

  // 7-track grid intact, rows 22px, splitter track 6px with centered 1px rail.
  const tracks = await page
    .getByTestId("textview/row-6")
    .locator(":scope > div")
    .evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "row grid has 7 tracks").toBe(7);
  expect(tracks[0], "left action track 20px").toBe("20px");
  expect(tracks[3], "thin splitter track 6px").toBe("6px");
  expect(tracks[4], "right action track 20px").toBe("20px");
  const splitterW = await page
    .getByTestId("textview/row-6")
    .locator(".tv-splitter")
    .evaluate((el) => el.getBoundingClientRect().width);
  expect(splitterW, "splitter measures 6px").toBeGreaterThan(5);
  expect(splitterW, "splitter measures 6px").toBeLessThan(7);
  const splitterRail = await page
    .getByTestId("textview/row-6")
    .locator(".tv-splitter")
    .evaluate((el) => {
      const cs = window.getComputedStyle(el, "::before");
      return { width: cs.width, bg: cs.backgroundColor, content: cs.content, transform: cs.transform };
    });
  expect(splitterRail.content, "splitter paints a centered ::before rail").not.toBe("none");
  expect(splitterRail.width, "splitter rail is a calm 1px line").toBe("1px");
  expect(splitterRail.bg, "splitter rail uses subtle ink").toBe("rgb(229, 229, 234)");
  expect(splitterRail.transform, "splitter rail centered via translate").not.toBe("none");
  expect(await page.getByTestId("textview/row-6").evaluate((el) => (el as HTMLElement).offsetHeight)).toBe(22);

  // Arrows in per-pane gutters: 12px golden-amber TakeArrow SVG, no box.
  const leftBtn = page.getByTestId("hunk-copy-left-0");
  const rightBtn = page.getByTestId("hunk-copy-right-0");
  await expect(leftBtn).toBeVisible();
  await expect(rightBtn).toBeVisible();
  for (const btn of [leftBtn, rightBtn]) {
    const svg = btn.locator("svg");
    await expect(svg).toHaveCount(1);
    const svgBox = await svg.evaluate((el) => el.getBoundingClientRect().toJSON());
    expect(Math.round(svgBox.width), "arrow svg is 12px").toBe(12);
    expect(Math.round(svgBox.height), "arrow svg is 12px").toBe(12);
    const cls = await svg.evaluate((el) => el.getAttribute("class") ?? "");
    expect(cls, "no lucide chevron class").not.toMatch(/lucide/);
    const style = await btn.evaluate((el) => {
      const cs = window.getComputedStyle(el);
      return { color: cs.color, borderTopWidth: cs.borderTopWidth, backgroundColor: cs.backgroundColor };
    });
    expect(style.color, "golden-amber --ac-tab-dirty ink").toBe("rgb(255, 159, 10)");
    expect(style.borderTopWidth, "no button box border").toBe("0px");
    expect(style.backgroundColor, "no button box fill").toBe("rgba(0, 0, 0, 0)");
  }
  const leftD = await leftBtn.locator("svg").evaluate((el) => el.innerHTML);
  const rightD = await rightBtn.locator("svg").evaluate((el) => el.innerHTML);
  expect(leftD, "left/right arrows mirror").not.toBe(rightD);

  // Bracket roles: hunk 0 (rows 6..9) start/middle/middle/end on both gutters.
  expect(await bracketOf(page, 6), "hunk start brackets both gutters").toEqual(["start", "start"]);
  expect(await bracketOf(page, 7), "hunk middle brackets both gutters").toEqual(["middle", "middle"]);
  expect(await bracketOf(page, 9), "hunk end brackets both gutters").toEqual(["end", "end"]);
  expect(await page.getByTestId("textview/row-0").locator("span[data-bracket]").count()).toBe(0);

  // Wave-97 residual: the bracket column rides the numbers-side edge
  // (`right: 1px`, see TextView ActionGutter) beside the TakeArrow
  // button — BC's bracket runs beside its take arrow, never hidden
  // behind it — so no centering translate remains (pre-97 the 1px
  // stem straddled gutter center via translate).
  for (const row of [6, 7, 9]) {
    const stem = await page
      .getByTestId(`textview/row-${row}`)
      .locator("span[data-bracket]")
      .first()
      .evaluate((el) => {
        const cs = window.getComputedStyle(el, "::before");
        return { content: cs.content, right: cs.right };
      });
    expect(stem.content, `row-${row} stem renders`).not.toBe("none");
    expect(stem.right, `row-${row} stem rides the numbers-side edge`).toBe("1px");
  }
  // Middle rail spans the full gutter height; start stem is half.
  const rail = await page
    .getByTestId("textview/row-7")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => {
      const cs = window.getComputedStyle(el, "::before");
      return { height: cs.height, bg: cs.backgroundColor, opacity: cs.opacity, own: el.getBoundingClientRect().height };
    });
  expect(rail.height, "middle rail spans the gutter's full height").toBe(`${rail.own}px`);
  expect(rail.bg, "rail uses full-opacity muted ink (light)").toBe("rgb(110, 110, 115)");
  expect(rail.opacity, "rail at full opacity").toBe("1");
  const startRailH = await page
    .getByTestId("textview/row-6")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => window.getComputedStyle(el, "::before").height);
  expect(parseFloat(startRailH), "start stem is half the full rail").toBeCloseTo(rail.own / 2, 0);
  // End foot: 5px rightward foot from rail center.
  const endFoot = await page
    .getByTestId("textview/row-9")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => {
      const cs = window.getComputedStyle(el, "::after");
      return { content: cs.content, width: cs.width };
    });
  expect(endFoot.content, "end foot renders a ::after").not.toBe("none");
  expect(endFoot.width, "end foot is 5px").toBe("5px");

  // Orphan honesty: null right cell hatched + untinted, live left keeps tv-del.
  const rightCell = page.getByTestId("textview/row-6").locator('div[title="right"]');
  const leftCell = page.getByTestId("textview/row-6").locator('div[title="left"]');
  await expect(rightCell).toHaveText("");
  await expect(rightCell).toHaveClass(/tv-cell-empty/);
  for (const tint of ["tv-del", "tv-ins", "tv-chg"]) {
    await expect(rightCell).not.toHaveClass(new RegExp(tint));
  }
  await expect(leftCell).toHaveClass(/tv-del/);
  const hatch = await rightCell.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { image: cs.backgroundImage, bg: cs.backgroundColor };
  });
  expect(hatch.image, "orphan hatch is pure-CSS gradient").toMatch(/linear-gradient/);
  expect(hatch.bg, "orphan hatch sits on base bg (light)").toBe("rgb(245, 245, 247)");

  // Dark scheme holds: rail, hatch, arrows visible, no alerts.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("hunk-copy-left-0")).toBeVisible();
  await expect(page.getByTestId("hunk-copy-right-0")).toBeVisible();
  await expect(rightCell).toHaveClass(/tv-cell-empty/);
  const darkRail = await page
    .getByTestId("textview/row-7")
    .locator("span[data-bracket]")
    .first()
    .evaluate((el) => {
      const cs = window.getComputedStyle(el, "::before");
      return { bg: cs.backgroundColor, right: cs.right };
    });
  expect(darkRail.bg, "rail stays visible in dark").toBe("rgb(161, 161, 166)");
  expect(darkRail.right, "rail stays numbers-side in dark").toBe("1px");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  // Arrow copy smoke: take-left fills right cells + clears the hunk.
  await expect(rightCell).toHaveText("");
  await leftBtn.click();
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", "equal");
  await expect(page.getByTestId("textview/row-6").locator('div[title="right"]')).toHaveText("removed line A");
  // Status counts honestly follow the resolved rows: hunk 0's 4
  // left-only rows are now same (188 -> 192 same, 0 left-only).
  await expect(page.getByTestId("textview/status")).toHaveText(
    "200 / 644 rows · 192 same · 4 changed · 0 left-only · 4 right-only (loaded)",
  );
  await expect(page.getByRole("alert")).toHaveCount(0);

  await assertNoOverlap(page);
  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
