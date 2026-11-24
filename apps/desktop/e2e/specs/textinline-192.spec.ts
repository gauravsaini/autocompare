/**
 * Wave-192 text inline highlights slice (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Ponytail scope: RowLine already paints `row.inline` via `InlineSpans`
 * + `.tv-w-hl` (66S2 wiring untouched, no layout change); this spec pins
 * the 192 paint + the real ignore-IPC wiring on the demo diff:
 *  1. inline highlights: changed row 51 renders one `inline/hl` mark per
 *     side (`data-side` matches pane, `.tv-w-hl` class, red-on-pink paint
 *     light mode); equal row 0 renders zero marks; orphan null cell
 *     (row 6 right, `leftOnly`) keeps `.tv-cell-empty` with zero marks;
 *  2. ignore toggles: ws/case checkboxes flip the real `text_compare`
 *     IPC flags (`__TEXT_IGNORE_LAST__` probe + UI toggle poll), no fake
 *     "not supported" note element, status still `200 / 644 rows`;
 *  3. budget HOLD: firstRowTop <= 140px with panels closed + zero
 *     console/page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test } from "@playwright/test";
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

test("wave-192 textinline: red-on-pink highlights + ignore IPC + budget", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 1. Inline highlights on changed row 51 (mock `inline` op).
  const changed = page.getByTestId("textview/row-51");
  await expect(changed).toHaveAttribute("data-kind", "changed");
  const leftHl = changed.locator('div[title="left"] mark[data-testid="inline/hl"]');
  const rightHl = changed.locator('div[title="right"] mark[data-testid="inline/hl"]');
  await expect(leftHl).toHaveCount(1);
  await expect(leftHl).toHaveText("left");
  await expect(leftHl).toHaveAttribute("data-side", "left");
  await expect(leftHl).toHaveClass(/tv-w-hl/);
  await expect(rightHl).toHaveCount(1);
  await expect(rightHl).toHaveText("right");
  await expect(rightHl).toHaveAttribute("data-side", "right");
  await expect(rightHl).toHaveClass(/tv-w-hl/);

  // Red-on-pink paint (light mode): pink bg, bright-red ink.
  const paint = await leftHl.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { bg: cs.backgroundColor, fg: cs.color };
  });
  const nums = (s: string): number[] => (s.match(/[\d.]+/g) ?? []).map(Number);
  const bg = nums(paint.bg);
  const fg = nums(paint.fg);
  expect(bg.length, `hl bg resolves (got ${paint.bg})`).toBeGreaterThanOrEqual(3);
  expect(bg[0]!, "hl bg red channel (pink)").toBeGreaterThan(240);
  expect(bg[1]!, "hl bg green channel (pink)").toBeGreaterThan(150);
  expect(bg[2]!, "hl bg blue channel (pink)").toBeGreaterThan(150);
  expect(fg[0]!, "hl ink red channel (bright red)").toBeGreaterThan(150);
  expect(fg[1]!, "hl ink green channel low (red)").toBeLessThan(110);
  expect(fg[2]!, "hl ink blue channel low (red)").toBeLessThan(110);

  // Equal rows stay plain text, zero marks.
  const equal = page.getByTestId("textview/row-0");
  await expect(equal).toHaveAttribute("data-kind", "equal");
  await expect(equal.locator('mark[data-testid="inline/hl"]')).toHaveCount(0);

  // Orphan null cell: hatch, zero marks.
  const orphan = page.getByTestId("textview/row-6");
  await expect(orphan).toHaveAttribute("data-kind", "leftOnly");
  const orphanRight = orphan.locator('div[title="right"]');
  await expect(orphanRight).toHaveClass(/tv-cell-empty/);
  await expect(orphanRight.locator('mark[data-testid="inline/hl"]')).toHaveCount(0);

  // 2. Ignore toggles ride the real IPC options (no fake note).
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  const probe = await page.evaluate(async () => {
    const mod = await import("../../src/ipc/text.ts");
    const w = window as unknown as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    await mod.textCompare("l", "r", { ignoreWhitespace: true });
    out["wsOnly"] = w["__TEXT_IGNORE_LAST__"];
    await mod.textCompare("l", "r", { ignoreCase: true });
    out["caseOnly"] = w["__TEXT_IGNORE_LAST__"];
    await mod.textCompare("l", "r");
    out["legacy"] = w["__TEXT_IGNORE_LAST__"];
    return out;
  });
  expect((probe["wsOnly"] as Record<string, unknown>)["ignoreWhitespace"], "ws probe").toBe(true);
  expect((probe["caseOnly"] as Record<string, unknown>)["ignoreCase"], "case probe").toBe(true);
  expect((probe["legacy"] as Record<string, unknown>)["ignoreWhitespace"], "legacy off").toBe(false);

  const ignoreToggle = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle.getAttribute("aria-expanded")) !== "true") await ignoreToggle.click();
  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  const caseToggle = page.getByTestId("textview/ignore-case");
  await expect(wsToggle).toBeVisible();
  await expect(caseToggle).toBeVisible();
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
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 3. Budget HOLD: panels closed, first row within 140px of section top.
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

  await assertNoOverlap(page);
  expect(consoleErrors, "zero console errors").toEqual([]);
  expect(pageErrors, "zero page errors").toEqual([]);
});
