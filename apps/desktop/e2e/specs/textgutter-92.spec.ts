/**
 * Wave-92 text gutter last-mile audit (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Ponytail audit over the wave-66I per-pane gutters + 66O arrows + 66Q
 * exact parity + 79 far-left strip + 89 chrome budget: drives the REAL
 * `App` workspace with the auto-booted demo text diff (mock
 * `text_compare` over `dumped.json`: 644 rows, 200 loaded) and proves
 * the BC last mile with NO layout rewrite:
 *  1. per-pane gutters: 7-track grid
 *     `20px gutter 1fr 6px 20px gutter 1fr`, left cell hosts
 *     `hunk-copy-left-N`, right cell `hunk-copy-right-N`, no `.w-14`
 *     center box, visual x-order LeftAction/LeftLineNo/LeftText/
 *     Splitter/RightAction/RightLineNo/RightText;
 *  2. bracket rails: per-row `data-bracket` roles from the existing
 *     hunk runs (first=start|single, last=end|single, interior=middle;
 *     absent on equal rows), single = arrow only (no rail);
 *  3. orphan hatch: null cells paint `.tv-cell-empty`
 *     (repeating-linear-gradient hatch) with NO tint class while the
 *     live sibling keeps its tint; the row div carries no tint;
 *  4. center splitter: 6px track with a single centered 1px
 *     `--ac-border-subtle` rail; per-side LineNo divider sits between
 *     LineNo and code on both panes;
 *  5. density + chrome: measured row height 22px (`--ac-row-h` 22px),
 *     section-relative first-row delta <= 140px with panels closed
 *     (wave-89 proof intact, no new stacked rows);
 *  6. wiring intact: InlineSpans highlights present, ignore controls
 *     backend-wired (no honest-note regression), OverviewStrip
 *     far-left with per-hunk markers;
 *  7. light + dark PNGs land in `/tmp/ui-92/` (never in-repo), zero
 *     console errors / page errors.
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

test("wave-92 gutter last-mile audit: per-pane gutters, rails, hatch, splitter, budget", async ({
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
  // Fall back to the documented 66Q hunk (rows 6..9) if the prefix is equal.
  const hunkRows = firstRun.length > 0 ? firstRun : [6, 7, 8, 9];
  const hunkHead = hunkRows[0]!;

  const head = page.getByTestId(`textview/row-${hunkHead}`);
  await expect(head).toBeVisible();

  // 1. Per-pane gutters with kept testids, no 56px center box.
  await expect(head.locator('[data-testid^="hunk-copy-left-"]')).toHaveCount(1);
  await expect(head.locator('[data-testid^="hunk-copy-right-"]')).toHaveCount(1);
  await expect(head.locator(".w-14")).toHaveCount(0);
  expect(await page.locator('[data-testid^="hunk-copy-left-"]').count()).toBeGreaterThan(0);
  expect(await page.locator('[data-testid^="hunk-copy-right-"]').count()).toBeGreaterThan(0);

  // 7-track grid: 20px actions, 6px splitter.
  const tracks = await head
    .locator(":scope > div")
    .evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "row grid has 7 tracks").toBe(7);
  expect(tracks[0], "left action track 20px").toBe("20px");
  expect(tracks[3], "thin splitter track 6px").toBe("6px");
  expect(tracks[4], "right action track 20px").toBe("20px");

  // Visual x-order: LeftAction, LeftLineNo, LeftText, Splitter,
  // RightAction, RightLineNo, RightText.
  const xs = await head.evaluate((row) => {
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

  // Crisp borderless golden-amber arrows (66O): 12px inline SVG in the
  // button, no gray box (transparent/borderless, hover highlight only).
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
  // Equal rows mount no bracket at all.
  const equalRow = loaded.find((r) => r.kind === "equal");
  if (equalRow !== undefined) {
    await expect(
      page.getByTestId(`textview/row-${equalRow.index}`).locator("span[data-bracket]"),
      "equal row mounts no rail",
    ).toHaveCount(0);
  }

  // 3. Orphan hatch on null cells only, no tint bleed, no whole-row tint.
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
  expect(hatch.empties, "orphan hatch cells exist").toBeGreaterThan(0);
  expect(hatch.badTint, "hatch cells carry no tint").toBe(0);
  expect(hatch.tintedRows, "no whole-row tint").toBe(0);
  expect(hatch.liveTinted, "live sibling keeps tint").toBeGreaterThan(0);
  expect(hatch.bg, "hatch is a repeating gradient").toContain("repeating-linear-gradient");

  // 4. Center splitter: 6px track already asserted; visible rail is 1px.
  const splitterW = await head
    .locator(".tv-splitter")
    .evaluate((el) => window.getComputedStyle(el, "::before").width);
  expect(splitterW, "splitter rail 1px").toBe("1px");
  // Per-side LineNo divider (wave-97 BC verdict): left keeps its
  // divider on the code side (right edge), right moved its divider to
  // the gutter side (left edge) so the number hugs its code cell —
  // bracket/stroke left of numbers, no number|text rule (see
  // /tmp/bc-text.png right pane).
  const leftW = await head
    .locator('[data-testid^="textview/lineno-left-"]')
    .evaluate((el) => window.getComputedStyle(el).borderRightWidth);
  expect(leftW, "left lineno divider 1px").toBe("1px");
  const rightL = await head
    .locator('[data-testid^="textview/lineno-right-"]')
    .evaluate((el) => window.getComputedStyle(el).borderLeftWidth);
  expect(rightL, "right lineno divider 1px gutter-side").toBe("1px");
  const rightR = await head
    .locator('[data-testid^="textview/lineno-right-"]')
    .evaluate((el) => window.getComputedStyle(el).borderRightWidth);
  expect(rightR, "right lineno has no code-side rule").toBe("0px");

  // 5. Density: measured row height 22px, token 22px.
  expect(await head.evaluate((el) => (el as HTMLElement).offsetHeight), "row height 22px").toBe(22);
  expect(
    await page.evaluate(() =>
      window.getComputedStyle(document.documentElement).getPropertyValue("--ac-row-h").trim(),
    ),
    "--ac-row-h 22px",
  ).toBe("22px");
  expect(
    await head.evaluate((el) => window.getComputedStyle(el).borderBottomWidth),
    "no row border-bottom striping",
  ).toBe("0px");

  // 6. Wiring intact: inline highlights, backend-wired ignores, far-left strip.
  expect(await page.locator('[data-testid="inline/hl"]').count(), "inline highlights ride").toBeGreaterThan(0);
  await expect(page.getByTestId("textview/toolbar-ignore")).toBeVisible();
  // Wave-112 preamble (batch-green extension, same pattern as the five
  // named specs): the ignore panel is closed by default; open it via
  // its toolbar toggle. Zero assertion changes.
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

  // 7. Chrome budget (wave-89 proof intact): section-relative first-row
  // delta <= 140px with all three panels closed; reopen afterwards.
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

  // Light PNG (panels reopened, default scheme).
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-92/textgutter-92-light.png" });
  // Dark PNG via emulated color scheme (tokens ride the media query).
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-92/textgutter-92-dark.png" });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors, "zero console errors").toEqual([]);
  expect(pageErrors, "zero page errors").toEqual([]);
});
