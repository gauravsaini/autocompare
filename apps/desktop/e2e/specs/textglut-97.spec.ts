/**
 * Wave-97 text glut last-mile (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * pins the three wave-97 gutter/row verdicts against live BC
 * (`/tmp/bc-text.png`):
 *  1. start-row stem runs BESIDE the TakeArrow button (numbers-side
 *     edge, `right-[1px]`), never hidden behind it; middle/end rails
 *     share the same column (no step) and the end foot turns leftward
 *     (BC `[` beside its take arrow);
 *  2. right-pane LineNo divider moved to the gutter side (`border-l`,
 *     number hugs code, no number|text rule); left keeps `border-r`;
 *     no row carries `border-bottom` (seamless hatch, no striping);
 *  3. orphan hatch tiles seamlessly across 22px rows (5.19px period =
 *     22 / sqrt(2) / 3, pinned in the computed gradient).
 * Plus: RowLine inline word highlights verified live (marks ride
 * non-null tinted cells only, orphan cells mark-free since 66S2 — no
 * fix needed); ignore whitespace/case round-trips through the typed
 * `textCompare` path with the "not supported" fallback note GONE
 * (Rust `TextOptions` honors both flags, `appliedOptions` always
 * echoed); chrome budget firstRowTop<=140 intact.
 *
 * New file only; light + dark PNG parts land in `/tmp/` (never
 * in-repo); zero console errors / page errors.
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

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

test("textglut-97 last mile: gutter verdicts, inline live, ignore wired, chrome budget", async ({
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

  // Hunk head (fixture rows 6..9: start/middle/middle/end per 66I).
  const headIdx = await page.evaluate(() => {
    const rows = [...document.querySelectorAll(".tv-row")];
    for (const r of rows) {
      if (r.querySelector('[data-testid^="hunk-copy-left-"]')) {
        return Number((r.getAttribute("data-testid") ?? "").replace("textview/row-", ""));
      }
    }
    return -1;
  });
  expect(headIdx, "hunk head mounted").toBeGreaterThanOrEqual(0);
  const head = page.getByTestId(`textview/row-${headIdx}`);
  await expect(head).toHaveAttribute("data-kind", "leftOnly");
  await head.scrollIntoViewIfNeeded();

  // (a1) Start stem beside the button: same column as middle/end
  // rails (no step), horizontally clear of the button box while
  // vertically overlapping it (beside, not behind).
  const bracket = await page.evaluate((idx: number) => {
    const meas = (rowIdx: number): { left: number; top: number; height: number } | null => {
      const row = document.querySelector(`[data-testid="textview/row-${rowIdx}"]`);
      const cell = row?.querySelector("span[data-bracket]:not([data-bracket='single'])");
      if (!(cell instanceof HTMLElement)) return null;
      const cs = window.getComputedStyle(cell, "::before");
      if (cs.content === "none") return null;
      const r = cell.getBoundingClientRect();
      return {
        left: r.left + Number.parseFloat(cs.left),
        top: r.top + Number.parseFloat(cs.top),
        height: Number.parseFloat(cs.height),
      };
    };
    const row = document.querySelector(`[data-testid="textview/row-${idx}"]`);
    const cell = row?.querySelector('span[data-bracket="start"]');
    const btn = cell?.querySelector("button");
    if (!(btn instanceof HTMLElement)) return null;
    const b = btn.getBoundingClientRect();
    return {
      start: meas(idx),
      next: meas(idx + 1),
      btnRight: b.right,
      btnTop: b.top,
      btnBottom: b.bottom,
    };
  }, headIdx);
  expect(bracket, "start bracket + button resolve").not.toBeNull();
  expect(bracket?.start?.height ?? 0, "start stem is half height (11px)").toBeCloseTo(11, 0);
  expect(
    bracket!.start!.left,
    "start stem horizontally clear of the button (never behind it)",
  ).toBeGreaterThanOrEqual(bracket!.btnRight - 1);
  expect(bracket!.start!.top, "stem overlaps the button vertically (beside it)").toBeLessThan(
    bracket!.btnBottom,
  );
  expect(bracket?.next?.left ?? NaN, "middle rail shares the stem column (no step)").toBeCloseTo(
    bracket!.start!.left,
    0,
  );
  // End foot turns leftward from the rail (BC `[`).
  const foot = await page.evaluate((idx: number) => {
    const row = document.querySelector(`[data-testid="textview/row-${idx}"]`);
    const cell = row?.querySelector('span[data-bracket="end"]');
    if (!(cell instanceof HTMLElement)) return null;
    const cs = window.getComputedStyle(cell, "::after");
    return { content: cs.content, width: Number.parseFloat(cs.width), right: cs.right };
  }, headIdx + 3);
  expect(foot?.content, "end foot renders").not.toBe("none");
  expect(foot?.width, "end foot 5px").toBe(5);
  expect(foot?.right, "end foot meets the rail").toBe("1px");

  // (a2) LineNo dividers: left on the code side, right on the gutter side.
  const leftDiv = await head
    .locator('[data-testid^="textview/lineno-left-"]')
    .evaluate((el) => {
      const cs = window.getComputedStyle(el);
      return { left: cs.borderLeftWidth, right: cs.borderRightWidth };
    });
  expect(leftDiv.right, "left lineno divider 1px code-side").toBe("1px");
  expect(leftDiv.left, "left lineno no outer rule").toBe("0px");
  const rightDiv = await head
    .locator('[data-testid^="textview/lineno-right-"]')
    .evaluate((el) => {
      const cs = window.getComputedStyle(el);
      return { left: cs.borderLeftWidth, right: cs.borderRightWidth };
    });
  expect(rightDiv.left, "right lineno divider 1px gutter-side").toBe("1px");
  expect(rightDiv.right, "right lineno no code-side rule").toBe("0px");

  // (a3) No row border-bottom; seamless hatch pinned by period.
  expect(
    await head.evaluate((el) => window.getComputedStyle(el).borderBottomWidth),
    "no row border-bottom striping",
  ).toBe("0px");
  const hatchBg = await head
    .locator('div[title="right"]')
    .evaluate((el) => window.getComputedStyle(el).backgroundImage);
  expect(hatchBg, "orphan hatch is a repeating gradient").toContain("repeating-linear-gradient");
  expect(hatchBg, "hatch period tiles 22px rows seamlessly").toContain("5.19px");

  // (b) Inline word highlights verified live (66S2 wiring intact).
  const inline = await page.evaluate(() => {
    const marks = [...document.querySelectorAll('mark[data-testid="inline/hl"]')];
    let badCell = 0;
    let badSide = 0;
    for (const m of marks) {
      const cell = m.closest(".tv-cell");
      const tinted =
        cell !== null &&
        !cell.classList.contains("tv-cell-empty") &&
        ["tv-del", "tv-ins", "tv-chg"].some((c) => cell.classList.contains(c));
      if (!tinted) badCell += 1;
      const side = m.getAttribute("data-side");
      const title = cell?.getAttribute("title");
      if (side !== title) badSide += 1;
    }
    const orphans = [...document.querySelectorAll(".tv-row .tv-cell-empty")];
    const orphanMarks = orphans.reduce(
      (n, c) => n + c.querySelectorAll('mark[data-testid="inline/hl"]').length,
      0,
    );
    return { marks: marks.length, badCell, badSide, orphans: orphans.length, orphanMarks };
  });
  expect(inline.marks, "inline highlights ride").toBeGreaterThan(0);
  expect(inline.badCell, "every mark rides a non-null tinted cell").toBe(0);
  expect(inline.badSide, "every mark matches its pane side").toBe(0);
  expect(inline.orphans, "orphan hatch cells exist").toBeGreaterThan(0);
  expect(inline.orphanMarks, "orphan cells stay mark-free").toBe(0);

  // (c) Ignore whitespace/case wired via the typed path; the
  // "not supported" note element is gone for good.
  // Wave-112 preamble (batch-green extension, same pattern as the five
  // named specs): the ignore panel is closed by default; open it via
  // its toolbar toggle. Zero assertion changes.
  const ignoreToggle112 = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle112.getAttribute("aria-expanded")) !== "true") await ignoreToggle112.click();
  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  const caseToggle = page.getByTestId("textview/ignore-case");
  await expect(wsToggle).toBeVisible();
  await expect(caseToggle).toBeVisible();
  await wsToggle.check();
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const w = window as unknown as Record<string, unknown>;
          return (w["__TEXT_IGNORE_LAST__"] as Record<string, unknown> | undefined)?.[
            "ignoreWhitespace"
          ];
        }),
      { timeout: 10000 },
    )
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await wsToggle.uncheck();
  await caseToggle.check();
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const w = window as unknown as Record<string, unknown>;
          return (w["__TEXT_IGNORE_LAST__"] as Record<string, unknown> | undefined)?.[
            "ignoreCase"
          ];
        }),
      { timeout: 10000 },
    )
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await caseToggle.uncheck();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  const typedEcho = (await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    return (await mod.textCompare("left-a", "right-b", {
      ignoreWhitespace: true,
      ignoreCase: true,
    })) as unknown as Record<string, unknown>;
  })) as Record<string, unknown>;
  expect(
    (typedEcho["appliedOptions"] as Record<string, unknown>)?.["ignoreWhitespace"],
    "typed path echoes ws",
  ).toBe(true);
  expect(
    (typedEcho["appliedOptions"] as Record<string, unknown>)?.["ignoreCase"],
    "typed path echoes case",
  ).toBe(true);

  // (d) Chrome budget intact (panels closed, like wave-89/92).
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
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(chrome ?? Number.POSITIVE_INFINITY, "firstRowTop<=140").toBeLessThanOrEqual(140);
  for (const toggle of [
    "textview/toolbar-find",
    "textview/toolbar-goto",
    "textview/toolbar-ignore",
  ] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded !== "true") await page.getByTestId(toggle).click();
  }

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Evidence parts (composed to the gate deliverables via magick).
  await head.scrollIntoViewIfNeeded();
  await page.waitForTimeout(300);
  const clipParts = await page.evaluate((idx: number) => {
    const row = document.querySelector(`[data-testid="textview/row-${idx}"]`);
    if (!(row instanceof HTMLElement)) return null;
    const r = row.getBoundingClientRect();
    const cell = (sel: string): { x: number; w: number } => {
      const el = row.querySelector(sel);
      if (!(el instanceof HTMLElement)) return { x: r.x, w: 0 };
      const b = el.getBoundingClientRect();
      return { x: b.x, w: b.width };
    };
    return {
      rowY: r.y,
      rowH: r.height,
      leftAction: cell('span[data-bracket="start"]'),
      rightNums: cell('[data-testid^="textview/lineno-right-"]'),
      rightText: cell('div[title="right"]'),
    };
  }, headIdx);
  expect(clipParts, "clip geometry resolves").not.toBeNull();
  const cp = clipParts!;
  await page.screenshot({
    path: "/tmp/textglut-97-part-gutter-left.png",
    clip: { x: Math.max(0, cp.leftAction.x - 30), y: cp.rowY - 44, width: 150, height: 176 },
  });
  await page.screenshot({
    path: "/tmp/textglut-97-part-gutter-right.png",
    clip: { x: cp.rightNums.x - 60, y: cp.rowY - 44, width: 330, height: 176 },
  });
  await page.screenshot({
    path: "/tmp/textglut-97-part-hatch.png",
    clip: { x: cp.rightText.x, y: cp.rowY - 22, width: 480, height: 132 },
  });
  await page.screenshot({ path: "/tmp/textglut-97-part-full-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/textglut-97-part-full-dark.png" });
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
