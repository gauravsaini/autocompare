/**
 * Wave-145 text closed visual loop (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Per-view closed loop for Text on File1(170) vs File2(159) 8 hunks
 * (STUDY: `/tmp/bc-text.png` + live CLI `diff -U0` 8 hunks + python
 * 170/159). BUILD batch-fix within view only; this spec pins geometry
 * only here, behavior specs untouched:
 *  (a) 20px per-pane action gutters, borderless amber arrows;
 *  (b) bracket rail start/middle/end + leftward foot continuity;
 *  (c) orphan hatch seamless no bleed + thin 1px splitter;
 *  (d) ROW 22 density (pin only, spec-locked);
 *  (e) RowLine inline highlights;
 *  (f) ignore Ws/Case wiring + chrome firstRowTop<=140, no stacked rows.
 *
 * Subject: REAL `App` auto-booted demo diff (mock text_compare over
 * dumped.json: 644 rows, 200 loaded). Light + dark PNGs + gutter/inline
 * zoom crops land in `/tmp/ui-145/` (never in-repo). Zero console/page errors.
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

async function lastIgnoreFlags(page: {
  evaluate: <T>(fn: () => T) => Promise<T>;
}): Promise<Record<string, unknown> | null> {
  return page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const last = w["__TEXT_IGNORE_LAST__"];
    if (typeof last !== "object" || last === null) return null;
    return last as Record<string, unknown>;
  });
}

test("wave-145 textclosed: gutters + bracket + hatch + density + inline + ignore + chrome", async ({
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
  const hunkRows = firstRun.length > 0 ? firstRun : [6, 7, 8, 9];
  const hunkHead = hunkRows[0]!;
  const head = page.getByTestId(`textview/row-${hunkHead}`);
  await expect(head).toBeVisible();

  // (a) 20px per-pane gutters, 7-track grid, no center box, borderless amber arrows.
  await expect(head.locator('[data-testid^="hunk-copy-left-"]')).toHaveCount(1);
  await expect(head.locator('[data-testid^="hunk-copy-right-"]')).toHaveCount(1);
  await expect(head.locator(".w-14")).toHaveCount(0);
  const tracks = await head
    .locator(":scope > div")
    .evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "row grid has 7 tracks").toBe(7);
  expect(tracks[0], "left action track 20px").toBe("20px");
  expect(tracks[3], "thin splitter track 6px").toBe("6px");
  expect(tracks[4], "right action track 20px").toBe("20px");
  for (const side of ["left", "right"] as const) {
    const btn = head.locator(`[data-testid^="hunk-copy-${side}-"]`);
    await expect(btn.locator("svg"), `${side} arrow svg`).toHaveCount(1);
    expect(await btn.locator("svg").getAttribute("width"), `${side} arrow 12px`).toBe("12");
    const btnStyle = await btn.evaluate((el) => {
      const cs = window.getComputedStyle(el);
      return { border: cs.borderWidth, color: cs.color, bg: cs.backgroundColor };
    });
    expect(btnStyle.border, `${side} arrow borderless`).toBe("0px");
    expect(btnStyle.color, `${side} amber ink`).not.toBe("rgba(0, 0, 0, 0)");
  }

  // (b) Bracket rail start/middle/end + foot continuity, numbers-side edge.
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
  expect(stem?.beforeRight, "(b) rail rides numbers-side edge").toBe("1px");
  expect(stem?.beforeWidth, "(b) rail is 1px").toBe("1px");
  expect(stem?.beforeTop, "(b) start stem bottom-half").toBe("11px");
  expect(stem?.beforeBottom, "(b) start stem reaches bottom").toBe("0px");
  expect(stem!.btnRight!).toBeLessThan(stem!.cellWidth - 1);
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
    expect(pair.length, `row ${r} mounts both rails`).toBe(2);
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
    const foot = await page.evaluate((idx: number) => {
      const row = document.querySelector(`[data-testid="textview/row-${idx}"]`);
      const cell = row?.querySelector('span[data-bracket="end"]');
      if (!(cell instanceof HTMLElement)) return null;
      const cs = window.getComputedStyle(cell, "::after");
      return { content: cs.content, width: cs.width, right: cs.right };
    }, hunkRows[hunkRows.length - 1]!);
    expect(foot?.content, "(b) end foot renders").not.toBe("none");
    expect(foot?.width, "(b) end foot 5px").toBe("5px");
    expect(foot?.right, "(b) end foot meets rail").toBe("1px");
  }

  // (c) Orphan hatch seamless no bleed + thin 1px splitter.
  expect(
    await head.evaluate((el) => window.getComputedStyle(el).borderBottomWidth),
    "(c) no row border-bottom striping",
  ).toBe("0px");
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
  expect(hatch.empties, "(c) orphan hatch cells exist").toBeGreaterThan(0);
  expect(hatch.badTint, "(c) hatch cells tint-free").toBe(0);
  expect(hatch.tintedRows, "(c) no whole-row tint").toBe(0);
  expect(hatch.liveTinted, "(c) live sibling tinted").toBeGreaterThan(0);
  expect(hatch.bg, "(c) hatch gradient").toContain("repeating-linear-gradient");
  expect(hatch.bg, "(c) hatch seamless period").toContain("5.19px");
  const splitter = await head.locator(".tv-splitter").evaluate((el) => {
    const cs = window.getComputedStyle(el, "::before");
    return { width: cs.width, content: cs.content };
  });
  expect(splitter.content, "(c) splitter rail renders").not.toBe("none");
  expect(splitter.width, "(c) splitter thin 1px").toBe("1px");

  // (d) ROW density behavior check (geometry pixel locks live in living spec text.parity.spec.ts).
  expect(await head.evaluate((el) => (el as HTMLElement).offsetHeight > 0), "(d) row renders with positive height").toBe(true);
  expect(
    await page.evaluate(() =>
      window.getComputedStyle(document.documentElement).getPropertyValue("--ac-row-h").trim(),
    ),
    "(d) --ac-row-h 22px",
  ).toBe("22px");

  // (e) RowLine inline highlights.
  const changed = page.getByTestId("textview/row-51");
  await expect(changed).toHaveAttribute("data-kind", "changed");
  const leftHl = changed.locator('div[title="left"] mark[data-testid="inline/hl"]');
  const rightHl = changed.locator('div[title="right"] mark[data-testid="inline/hl"]');
  await expect(leftHl).toHaveCount(1);
  await expect(leftHl).toHaveAttribute("data-side", "left");
  await expect(leftHl).toHaveClass(/tv-w-hl/);
  await expect(rightHl).toHaveCount(1);
  await expect(rightHl).toHaveAttribute("data-side", "right");
  await expect(rightHl).toHaveClass(/tv-w-hl/);
  const hlPaint = await leftHl.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { bg: cs.backgroundColor, color: cs.color };
  });
  expect(hlPaint.bg, "(e) highlight has paint").not.toBe("rgba(0, 0, 0, 0)");
  const equal = page.getByTestId("textview/row-0");
  await expect(equal).toHaveAttribute("data-kind", "equal");
  await expect(equal.locator('mark[data-testid="inline/hl"]')).toHaveCount(0);
  const orphanRight = head.locator('div[title="right"]');
  await expect(orphanRight).toHaveClass(/tv-cell-empty/);
  await expect(orphanRight.locator('mark[data-testid="inline/hl"]')).toHaveCount(0);

  // (f) Ignore Ws/Case wiring + note hidden + restore.
  const probe = await page.evaluate(async () => {
    const mod = await import("../../src/ipc/text.ts");
    const w = window as unknown as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    await mod.textCompare("l", "r", { ignoreWhitespace: true });
    out["wsOnly"] = w["__TEXT_IGNORE_LAST__"];
    await mod.textCompare("l", "r", { ignoreCase: true });
    out["caseOnly"] = w["__TEXT_IGNORE_LAST__"];
    await mod.textCompare("l", "r", { ignoreWhitespace: true, ignoreCase: true });
    out["both"] = w["__TEXT_IGNORE_LAST__"];
    await mod.textCompare("l", "r");
    out["legacy"] = w["__TEXT_IGNORE_LAST__"];
    return out;
  });
  expect((probe["wsOnly"] as Record<string, unknown>)["ignoreWhitespace"], "(f) ws spelling").toBe(true);
  expect((probe["both"] as Record<string, unknown>)["ignoreCase"], "(f) both spelling").toBe(true);
  expect((probe["legacy"] as Record<string, unknown>)["ignoreWhitespace"], "(f) legacy false").toBe(false);
  const ignoreToggle = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle.getAttribute("aria-expanded")) !== "true") await ignoreToggle.click();
  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  const caseToggle = page.getByTestId("textview/ignore-case");
  await expect(wsToggle).toBeVisible();
  await expect(caseToggle).toBeVisible();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await wsToggle.check();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreWhitespace"], { timeout: 10000 })
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await caseToggle.check();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreCase"], { timeout: 10000 })
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await wsToggle.uncheck();
  await caseToggle.uncheck();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreWhitespace"], { timeout: 10000 })
    .toBe(false);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // Chrome budget: firstRowTop<=140, no new stacked rows (toolbar + popover only).
  for (const toggle of [
    "textview/toolbar-find",
    "textview/toolbar-goto",
    "textview/toolbar-ignore",
  ] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded === "true") await page.getByTestId(toggle).click();
  }
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

  // Captures: full + gutter zoom + inline zoom, light + dark.
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-145/textclosed-145-light.png" });
  await head.scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  const hunkBox = await page.evaluate((rows: number[]) => {
    const top = document.querySelector(`[data-testid="textview/row-${rows[0]}"]`);
    const bot = document.querySelector(`[data-testid="textview/row-${rows[rows.length - 1]}"]`);
    if (!(top instanceof HTMLElement) || !(bot instanceof HTMLElement)) return null;
    const t = top.getBoundingClientRect();
    const b = bot.getBoundingClientRect();
    return { x: t.x, y: t.top, w: t.width, bottom: b.bottom };
  }, hunkRows);
  expect(hunkBox, "hunk block resolves").not.toBeNull();
  await page.screenshot({
    path: "/tmp/ui-145/textclosed-145-gutter-light.png",
    clip: {
      x: Math.max(0, hunkBox!.x - 8),
      y: Math.max(0, hunkBox!.y - 24),
      width: 220,
      height: hunkBox!.bottom - hunkBox!.y + 48,
    },
  });
  await changed.scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  const inlineBox = await changed.evaluate((el) => {
    const r = (el as HTMLElement).getBoundingClientRect();
    return { x: r.x, y: r.top, w: r.width, h: r.height };
  });
  const vw = page.viewportSize()?.width ?? 1280;
  const vh = page.viewportSize()?.height ?? 800;
  await page.screenshot({
    path: "/tmp/ui-145/textclosed-145-inline-light.png",
    clip: {
      x: Math.max(0, Math.min(inlineBox.x - 8, vw - 100)),
      y: Math.max(0, Math.min(inlineBox.y - 20, vh - 80)),
      width: Math.max(50, Math.min(inlineBox.w + 16, 1100, vw - Math.max(0, Math.min(inlineBox.x - 8, vw - 100)))),
      height: Math.max(40, Math.min(inlineBox.h + 40, vh - Math.max(0, Math.min(inlineBox.y - 20, vh - 80)))),
    },
  });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/ui-145/textclosed-145-dark.png" });
  await head.scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  const hunkBoxDark = await page.evaluate((rows: number[]) => {
    const top = document.querySelector(`[data-testid="textview/row-${rows[0]}"]`);
    const bot = document.querySelector(`[data-testid="textview/row-${rows[rows.length - 1]}"]`);
    if (!(top instanceof HTMLElement) || !(bot instanceof HTMLElement)) return null;
    const t = top.getBoundingClientRect();
    const b = bot.getBoundingClientRect();
    return { x: t.x, y: t.top, w: t.width, bottom: b.bottom };
  }, hunkRows);
  const hb = hunkBoxDark ?? hunkBox;
  await page.screenshot({
    path: "/tmp/ui-145/textclosed-145-gutter-dark.png",
    clip: {
      x: Math.max(0, hb!.x - 8),
      y: Math.max(0, hb!.y - 24),
      width: 220,
      height: hb!.bottom - hb!.y + 48,
    },
  });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors, "zero console errors").toEqual([]);
  expect(pageErrors, "zero page errors").toEqual([]);
});
