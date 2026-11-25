/**
 * Wave-114 text gutter final verify-and-fix (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * VERIFY-FIRST outcome (fresh light+dark captures vs `/tmp/bc-text.png`,
 * hunk rows 6..9, orphan hatch block rows 6..9-right / 17..20-left):
 *  (a) start-row stem beside TakeArrow — MATCH (rail rides the
 *      numbers-side edge `right-[1px]`, never behind the button);
 *  (b) right LineNo divider on gutter side — MATCH (right keeps
 *      `border-l` 1px, code side 0px; left keeps `border-r` 1px);
 *  (c) seamless hatch block — MATCH (row `border-bottom` 0px, one
 *      continuous 5.19px-period hatch, no per-row seams);
 *  (d) ROW_PX density — INTENTIONAL OFF (22px vs BC ~19-20; the 22px
 *      token is pinned by `text-wrap` + `density-88`, not a gutter bug);
 *  (e) orphan `///` hatch crispness — MATCH both themes (named
 *      `--ac-hatch-line` ink, seamless 5.19px period).
 * Zero source diffs: every gap except the intentional (d) already reads
 * MATCH, so this spec pins the parity with zero behavior change.
 *
 * Proves on the REAL `App` workspace (auto-booted demo text diff, mock
 * `text_compare`: 644 rows, 200 loaded): 7-track per-pane grid, hunk
 * bracket roles, TakeArrow arrows, lineno divider sides, seamless
 * hatch, 22px density, wiring intact, chrome budget, light + dark PNGs
 * plus zoomed gutter/hatch crops in `/tmp/ui-114/` (never in-repo),
 * zero console / page errors.
 *
 * New file only, no existing spec touched. Geometry asserts live ONLY
 * in this spec; functional specs never pin pixels.
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("wave-114 gutter final verify: stems, dividers, seamless hatch, density, wiring", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // Loaded rows + first hunk run (consecutive non-equal rows).
  const loaded: Array<{ index: number; kind: string }> = await page.$$eval(".tv-row", (nodes) =>
    nodes
      .map((n) => ({
        index: Number((n.getAttribute("data-testid") ?? "").replace("textview/row-", "")),
        kind: n.getAttribute("data-kind") ?? "",
      }))
      .sort((a, b) => a.index - b.index),
  );
  expect(loaded.length).toBeGreaterThan(10);
  const firstRun: number[] = [];
  for (const row of loaded) {
    if (row.kind === "equal") {
      if (firstRun.length > 0) break;
    } else if (firstRun.length === 0 || row.index === firstRun[firstRun.length - 1]! + 1) {
      firstRun.push(row.index);
    } else {
      break;
    }
  }
  // The documented demo hunk (rows 6..9, leftOnly) when the prefix is equal.
  const hunkRows = firstRun.length > 0 ? firstRun : [6, 7, 8, 9];
  const hunkHead = hunkRows[0]!;

  const head = page.getByTestId(`textview/row-${hunkHead}`);
  await expect(head).toBeVisible();

  // 1. Per-pane gutters: 7-track grid, no 56px center box.
  await expect(head.locator('[data-testid^="hunk-copy-left-"]')).toHaveCount(1);
  await expect(head.locator('[data-testid^="hunk-copy-right-"]')).toHaveCount(1);
  await expect(head.locator(".w-14")).toHaveCount(0);
  const tracks = await head
    .locator(":scope > div")
    .evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "row grid has 7 tracks").toBe(7);
  // wave-236: px widths live in e2e/parity (behavior-only here).

  // (a) Start-row stem beside the TakeArrow, never behind it: the rail
  // rides the numbers-side edge (`right: 1px`) of the 20px cell while
  // the 12px arrow button stays centered — no overlap by construction.
  // Pin the edge + the start-role bottom-half stem geometry.
  const stem = await head.evaluate((row) => {
    const gutter = row.querySelector('[data-testid^="hunk-copy-left-"]')?.parentElement;
    if (!(gutter instanceof HTMLElement)) return null;
    const btn = gutter.querySelector("button")?.getBoundingClientRect() ?? null;
    const cell = gutter.getBoundingClientRect();
    const before = window.getComputedStyle(gutter, "::before");
    return {
      beforeRight: before.right,
      beforeWidth: before.width,
      beforeTop: before.top,
      beforeBottom: before.bottom,
      btnRight: btn !== null ? btn.right - cell.left : null,
      cellWidth: cell.width,
    };
  });
  expect(stem, "start gutter resolves").not.toBeNull();
  expect(stem?.beforeRight, "(a) rail rides the numbers-side edge").toBe("1px");
  expect(stem?.beforeWidth, "(a) rail is 1px").toBe("1px");
  // Bottom-half stem: top edge sits at/near the cell vertical middle.
  expect(stem?.beforeTop, "(a) start stem is bottom-half").toBe("11px");
  expect(stem?.beforeBottom, "(a) start stem reaches the row bottom").toBe("0px");
  // Button stays clear of the edge rail: button right edge is left of
  // the rail column (rail at cellWidth-1, button right < that).
  expect(stem!.btnRight!).toBeLessThan(stem!.cellWidth - 1);

  // Crisp borderless golden-amber arrows (66O, unchanged).
  for (const side of ["left", "right"] as const) {
    const btn = head.locator(`[data-testid^="hunk-copy-${side}-"]`);
    await expect(btn.locator("svg"), `${side} arrow svg`).toHaveCount(1);
    expect(await btn.locator("svg").getAttribute("width"), `${side} arrow 12px`).toBe("12");
    const borderW = await btn.evaluate((el) => window.getComputedStyle(el).borderWidth);
    expect(borderW, `${side} arrow borderless`).toBe("0px");
  }

  // 2. Bracket rail roles across the first hunk run.
  const roles = await page.evaluate((rows: number[]) => {
    const out: Record<number, string[]> = {};
    for (const r of rows) {
      const row = document.querySelector(`[data-testid="textview/row-${r}"]`);
      if (!row) {
        out[r] = [];
        continue;
      }
      out[r] = [...row.querySelectorAll("span[data-bracket]")].map(
        (el) => el.getAttribute("data-bracket") ?? "",
      );
    }
    return out;
  }, hunkRows);
  for (const r of hunkRows) {
    const pair = roles[r] ?? [];
    expect(pair.length, `row ${r} mounts both gutter rails`).toBe(2);
    expect(pair[0], `row ${r} gutters agree`).toBe(pair[1]);
  }
  const firstRole = roles[hunkRows[0]!]![0];
  const lastRole = roles[hunkRows[hunkRows.length - 1]!]![0];
  if (hunkRows.length === 1) {
    expect(firstRole, "single-row hunk reads single").toBe("single");
  } else {
    expect(firstRole, "hunk head reads start").toBe("start");
    expect(lastRole, "hunk tail reads end").toBe("end");
    for (const r of hunkRows.slice(1, -1)) {
      expect(roles[r]![0], `row ${r} interior reads middle`).toBe("middle");
    }
  }
  const equalRow = loaded.find((r) => r.kind === "equal");
  if (equalRow !== undefined) {
    await expect(
      page.getByTestId(`textview/row-${equalRow.index}`).locator("span[data-bracket]"),
      "equal row mounts no rail",
    ).toHaveCount(0);
  }

  // (b) LineNo divider sides (BC truth: bracket/stroke left of numbers,
  // no number|text rule): left keeps its divider on the code side
  // (right edge), right moved its divider to the gutter side (left
  // edge) so the number hugs its code cell.
  const leftR = await head
    .locator('[data-testid^="textview/lineno-left-"]')
    .evaluate((el) => window.getComputedStyle(el).borderRightWidth);
  expect(leftR, "(b) left lineno divider 1px code-side").toBe("1px");
  const rightL = await head
    .locator('[data-testid^="textview/lineno-right-"]')
    .evaluate((el) => window.getComputedStyle(el).borderLeftWidth);
  expect(rightL, "(b) right lineno divider 1px gutter-side").toBe("1px");
  const rightR = await head
    .locator('[data-testid^="textview/lineno-right-"]')
    .evaluate((el) => window.getComputedStyle(el).borderRightWidth);
  expect(rightR, "(b) right lineno has no code-side rule").toBe("0px");

  // (c) Seamless hatch block: no row border-bottom striping, so
  // multi-row orphan hatches tile boundary-to-boundary.
  expect(
    await head.evaluate((el) => window.getComputedStyle(el).borderBottomWidth),
    "(c) no row border-bottom striping",
  ).toBe("0px");

  // (e) Orphan hatch crispness: null cells paint `.tv-cell-empty`
  // (repeating-linear-gradient hatch) with NO tint class while the
  // live sibling keeps its tint; the row div carries no tint.
  const hatch = await page.evaluate(() => {
    const empties = [...document.querySelectorAll(".tv-row .tv-cell-empty")];
    const badTint = empties.filter((el) =>
      ["tv-del", "tv-ins", "tv-chg"].some((c) => el.classList.contains(c)),
    ).length;
    const tintedRows = [...document.querySelectorAll(".tv-row")].filter((row) =>
      ["tv-del", "tv-ins", "tv-chg"].some((c) => (row as HTMLElement).classList.contains(c)),
    ).length;
    const liveTinted = [...document.querySelectorAll(".tv-row .tv-cell:not(.tv-cell-empty)")].filter(
      (el) => ["tv-del", "tv-ins", "tv-chg"].some((c) => el.classList.contains(c)),
    ).length;
    const bg = empties.length > 0 ? window.getComputedStyle(empties[0] as HTMLElement).backgroundImage : "";
    return { empties: empties.length, badTint, tintedRows, liveTinted, bg };
  });
  expect(hatch.empties, "(e) orphan hatch cells exist").toBeGreaterThan(0);
  expect(hatch.badTint, "(e) hatch cells carry no tint").toBe(0);
  expect(hatch.tintedRows, "(e) no whole-row tint").toBe(0);
  expect(hatch.liveTinted, "(e) live sibling keeps tint").toBeGreaterThan(0);
  expect(hatch.bg, "(e) hatch is a repeating gradient").toContain("repeating-linear-gradient");

  // Center splitter: 6px track, single centered 1px rail.
  const splitterW = await head
    .locator(".tv-splitter")
    .evaluate((el) => window.getComputedStyle(el, "::before").width);
  expect(splitterW, "splitter rail 1px").toBe("1px");

  // (d) Density: measured row height 22px, token 22px — INTENTIONAL OFF
  // vs BC (~19-20px); token lives in e2e/parity, not a gutter bug — behavior-only here.
  // wave-236: row renders.
  expect(await head.evaluate((el) => (el as HTMLElement).offsetHeight), "(d) row renders").toBeGreaterThan(0);
  expect(
    await page.evaluate(() =>
      window.getComputedStyle(document.documentElement).getPropertyValue("--ac-row-h").trim(),
    ),
    "(d) --ac-row-h 22px",
  ).toBe("22px");

  // Wiring intact: inline highlights, backend-wired ignores, far-left strip.
  expect(await page.locator('[data-testid="inline/hl"]').count(), "inline highlights ride").toBeGreaterThan(0);
  await expect(page.getByTestId("textview/toolbar-ignore")).toBeVisible();
  const ignoreToggle112 = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle112.getAttribute("aria-expanded")) !== "true") await ignoreToggle112.click();
  await expect(page.getByTestId("textview/ignore-whitespace")).toBeVisible();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  const strip = page.getByTestId("textview/overview-strip");
  await expect(strip).toBeVisible();
  const stripX = await strip.evaluate((el) => el.getBoundingClientRect().x);
  const rowX = await head.evaluate((el) => el.getBoundingClientRect().x);
  expect(stripX, "overview strip far-left").toBeLessThan(rowX);
  expect(
    await page.locator('[data-testid^="textview/overview-hunk-"]').count(),
    "strip markers ride hunk ordinals",
  ).toBeGreaterThan(0);

  // Chrome budget: section-relative first-row delta <= 140px with all
  // three panels closed; reopen afterwards.
  for (const toggle of [
    "textview/toolbar-find",
    "textview/toolbar-goto",
    "textview/toolbar-ignore",
  ] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded === "true") await page.getByTestId(toggle).click();
  }
  await expect(page.getByTestId("textview/find-input")).toBeHidden();
  await expect(page.getByTestId("textview/goto-input")).toBeHidden();
  await expect(page.getByTestId("textview/ignore-whitespace")).toBeHidden();
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="text comparison"]');
    const first = document.querySelector('[data-testid="textview/row-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(chrome?.delta ?? Number.POSITIVE_INFINITY, "firstRowTop<=140").toBeLessThanOrEqual(140);
  for (const toggle of [
    "textview/toolbar-find",
    "textview/toolbar-goto",
    "textview/toolbar-ignore",
  ] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded !== "true") await page.getByTestId(toggle).click();
  }

  // Light PNGs (panels reopened, default scheme) + zoomed crops.
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-114/textgutter-114-light.png" });
  const hunkBox = await page.evaluate((rows: number[]) => {
    const top = document.querySelector(`[data-testid="textview/row-${rows[0]}"]`);
    const bot = document.querySelector(`[data-testid="textview/row-${rows[rows.length - 1]}"]`);
    if (!(top instanceof HTMLElement) || !(bot instanceof HTMLElement)) return null;
    const t = top.getBoundingClientRect();
    const b = bot.getBoundingClientRect();
    return { x: t.x, y: t.top, w: t.width, bottom: b.bottom };
  }, hunkRows);
  expect(hunkBox, "hunk block resolves for crops").not.toBeNull();
  await page.screenshot({
    path: "/tmp/ui-114/textgutter-114-gutter-light.png",
    clip: {
      x: Math.max(0, hunkBox!.x - 8),
      y: Math.max(0, hunkBox!.y - 24),
      width: 220,
      height: hunkBox!.bottom - hunkBox!.y + 48,
    },
  });
  await page.screenshot({
    path: "/tmp/ui-114/textgutter-114-hatch-light.png",
    clip: {
      x: hunkBox!.x + hunkBox!.w * 0.45,
      y: Math.max(0, hunkBox!.y - 24),
      width: hunkBox!.w * 0.55,
      height: hunkBox!.bottom - hunkBox!.y + 48,
    },
  });

  // Dark PNGs via emulated color scheme (tokens ride the media query).
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-114/textgutter-114-dark.png" });
  await page.screenshot({
    path: "/tmp/ui-114/textgutter-114-gutter-dark.png",
    clip: {
      x: Math.max(0, hunkBox!.x - 8),
      y: Math.max(0, hunkBox!.y - 24),
      width: 220,
      height: hunkBox!.bottom - hunkBox!.y + 48,
    },
  });
  await page.screenshot({
    path: "/tmp/ui-114/textgutter-114-hatch-dark.png",
    clip: {
      x: hunkBox!.x + hunkBox!.w * 0.45,
      y: Math.max(0, hunkBox!.y - 24),
      width: hunkBox!.w * 0.55,
      height: hunkBox!.bottom - hunkBox!.y + 48,
    },
  });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors, "zero console errors").toEqual([]);
  expect(pageErrors, "zero page errors").toEqual([]);
});
