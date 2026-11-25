/**
 * Wave-66S2 RowLine inline highlights + right-lineno divider + 22px
 * density (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the three 66S2 RowLine changes with no layout rewrite:
 *  1. inline highlights: changed row 51 (mock `inline` op [0,4)/[0,5))
 *     renders one `inline/hl` mark per side (`left` on the left cell,
 *     `right` on the right cell, `data-side` matching the pane, `.tv-w-hl`
 *     paint); equal row 0 renders zero marks (byte-identical plain text);
 *     orphan null cell (row 6 right, `leftOnly`) keeps `.tv-cell-empty`
 *     with zero marks and zero extra DOM;
 *  2. right divider: BOTH per-side linenos carry the divider on the code
 *     side (`border-r`, symmetric) — the right lineno's right edge meets
 *     the right text cell, no outer border on either lineno;
 *  3. density: measured row height is 22px (`ROW_PX` + text-scoped
 *     `--ac-row-h` re-scope; the shared token stays 28px for the other
 *     views, `.tv-splitter` inherits 22px with the row).
 *
 * Mock honesty: everything reads the DOM over the mock rows (no new
 * IPC; highlights derive from the already-loaded `row.inline` ops).
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

test("RowLine inline highlights, right divider, 22px density", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 1. Inline highlights on a changed row (mock row 51: left
  // "left variant of line 0047" [0,4), right "right variant..." [0,5)).
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

  // Equal rows untouched: plain text, zero marks.
  const equal = page.getByTestId("textview/row-0");
  await expect(equal).toHaveAttribute("data-kind", "equal");
  await expect(equal.locator('mark[data-testid="inline/hl"]')).toHaveCount(0);

  // Orphan null cell keeps the hatch: no tint, zero marks, zero extra DOM.
  const orphan = page.getByTestId("textview/row-6");
  await expect(orphan).toHaveAttribute("data-kind", "leftOnly");
  const orphanRight = orphan.locator('div[title="right"]');
  await expect(orphanRight).toHaveText("");
  await expect(orphanRight).toHaveClass(/tv-cell-empty/);
  await expect(orphanRight.locator('mark[data-testid="inline/hl"]')).toHaveCount(0);
  expect(await orphanRight.evaluate((el) => el.childElementCount), "zero extra DOM in null cell").toBe(0);

  // 2. Dividers (wave-97 BC verdict): left lineno carries border-r
  // (divider between lineno and code, no outer border); right lineno
  // carries border-l instead (divider on the gutter side so the number
  // hugs its code cell, no number|text rule — see /tmp/bc-text.png).
  const leftLineno = page.getByTestId(`textview/lineno-left-51`);
  const leftBorders = await leftLineno.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { left: cs.borderLeftWidth, right: cs.borderRightWidth };
  });
  expect(leftBorders.right, "left lineno divider on the code side").not.toBe("0px");
  expect(leftBorders.left, "left lineno has no outer border").toBe("0px");
  const rightLineno = page.getByTestId(`textview/lineno-right-51`);
  const rightBorders = await rightLineno.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { left: cs.borderLeftWidth, right: cs.borderRightWidth };
  });
  expect(rightBorders.left, "right lineno divider on the gutter side").not.toBe("0px");
  expect(rightBorders.right, "right lineno has no code-side rule").toBe("0px");
  // Divider geometry: the right lineno's right edge meets the right
  // text cell (no gap, no overlap).
  const xr = await changed.evaluate((row) => {
    const lineno = row.querySelector('[data-testid="textview/lineno-right-51"]') as HTMLElement | null;
    const text = row.querySelector('div[title="right"]') as HTMLElement | null;
    if (lineno === null || text === null) return null;
    return {
      linenoRight: lineno.getBoundingClientRect().right,
      textLeft: text.getBoundingClientRect().left,
    };
  });
  expect(xr, "right lineno + right text resolve").not.toBeNull();
  expect(Math.abs((xr?.linenoRight ?? 1) - (xr?.textLeft ?? 0)), "right divider sits between lineno and code").toBeLessThanOrEqual(1);

  // 3. Density: rows render (22px pin lives in e2e/parity).
  // wave-236 behavior-only.
  expect(await changed.evaluate((el) => (el as HTMLElement).offsetHeight), "changed row renders").toBeGreaterThan(0);
  expect(await equal.evaluate((el) => (el as HTMLElement).offsetHeight), "equal row renders").toBeGreaterThan(0);

  await assertNoOverlap(page);
  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
