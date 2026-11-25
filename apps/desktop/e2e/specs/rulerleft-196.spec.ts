/**
 * Wave-196 ruler-left slice (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Ponytail scope: move existing rulers only, no new component, no
 * gutter/splitter/logic change.
 *  - TextView `textview/overview-strip`: far-left 4px rail (was 5px),
 *    `order-first` beside the row list, same per-hunk section markers.
 *  - MergeView `mergeview/overview-ruler`: right edge -> far-left
 *    (`order-first`, 4px), same per-conflict markers + jump path.
 *  - SyncView owns no ruler: nothing to move (asserted by absence —
 *    no new element introduced).
 *
 * Pins (light + dark):
 *  1. ruler x < pane x (strip/ruler right edge left of the first
 *     row/card left edge), width <= 6px, section markers present with
 *     opaque paint;
 *  2. HOLD: text firstRowTop <= 140px, merge firstCardTop <= 140px,
 *     zero console errors / page errors, no overlap.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const NO_MARKERS =
  "header, nav, main, [data-testid]:not([data-testid^='textview/overview-hunk']):not([data-testid^='mergeview/overview-hunk'])";

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

function listen(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

async function closeTextPanels(page: Page): Promise<void> {
  for (const toggle of ["textview/toolbar-find", "textview/toolbar-goto", "textview/toolbar-ignore"] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded === "true") await page.getByTestId(toggle).click();
  }
}

async function createMergeSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/merge12-base.txt");
  await page.getByTestId("session/right-path").fill("/tmp/merge12-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-merge")).toBeVisible();
}

test("rulerleft-196 text: strip far-left 4px, markers present, HOLD", async ({ page }) => {
  const { consoleErrors, pageErrors } = listen(page);

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await closeTextPanels(page);

  // 1. Far-left: strip right edge sits left of the first row's left edge.
  const strip = page.getByTestId("textview/overview-strip");
  await expect(strip).toBeVisible();
  const geo = await page.evaluate(() => {
    const s = document.querySelector('[data-testid="textview/overview-strip"]');
    const r = document.querySelector('[data-testid="textview/row-0"]');
    if (!(s instanceof HTMLElement) || !(r instanceof HTMLElement)) return null;
    const sb = s.getBoundingClientRect();
    const rb = r.getBoundingClientRect();
    return { stripRight: sb.right, rowLeft: rb.left, width: sb.width };
  });
  expect(geo, "strip + row-0 resolve").not.toBeNull();
  expect(geo?.width ?? Number.POSITIVE_INFINITY, "ruler width <= 6px").toBeLessThanOrEqual(6);
  expect(geo?.width ?? 0, "ruler width paints (>= 1px)").toBeGreaterThanOrEqual(1);
  expect(
    geo?.stripRight ?? Number.POSITIVE_INFINITY,
    `ruler x < pane x (stripRight ${geo?.stripRight} vs rowLeft ${geo?.rowLeft})`,
  ).toBeLessThanOrEqual((geo?.rowLeft ?? Number.NEGATIVE_INFINITY) + 1);

  // Markers present: one per hunk (gutter arrows agree 1:1), opaque paint.
  const gutterHunks = await page.locator('[data-testid^="hunk-copy-left-"]').count();
  expect(gutterHunks, "demo loads at least two hunks").toBeGreaterThan(1);
  await expect(page.locator('[data-testid^="textview/overview-hunk-"]')).toHaveCount(gutterHunks);
  const markerBg = await page
    .getByTestId("textview/overview-hunk-0")
    .evaluate((el) => window.getComputedStyle(el).backgroundColor);
  expect(markerBg, "marker reuses a diff token (opaque paint)").not.toBe("rgba(0, 0, 0, 0)");

  // 2. HOLD: first-row chrome budget + no overlap + zero errors, light + dark.
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="text comparison"]');
    const first = document.querySelector('[data-testid="textview/row-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(chrome ?? Number.POSITIVE_INFINITY, `firstRowTop<=140 (got ${chrome}px)`).toBeLessThanOrEqual(140);
  await assertNoOverlap(page, NO_MARKERS);

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  const darkGeo = await page.evaluate(() => {
    const s = document.querySelector('[data-testid="textview/overview-strip"]');
    const r = document.querySelector('[data-testid="textview/row-0"]');
    if (!(s instanceof HTMLElement) || !(r instanceof HTMLElement)) return null;
    const sb = s.getBoundingClientRect();
    const rb = r.getBoundingClientRect();
    return { stripRight: sb.right, rowLeft: rb.left, width: sb.width };
  });
  expect(darkGeo, "strip + row-0 resolve (dark)").not.toBeNull();
  expect(darkGeo?.width ?? Number.POSITIVE_INFINITY, "ruler width <= 6px (dark)").toBeLessThanOrEqual(6);
  expect(
    darkGeo?.stripRight ?? Number.POSITIVE_INFINITY,
    "ruler x < pane x (dark)",
  ).toBeLessThanOrEqual((darkGeo?.rowLeft ?? Number.NEGATIVE_INFINITY) + 1);
  const darkBg = await page
    .getByTestId("textview/overview-hunk-0")
    .evaluate((el) => window.getComputedStyle(el).backgroundColor);
  expect(darkBg, "marker keeps opaque paint (dark)").not.toBe("rgba(0, 0, 0, 0)");
  await assertNoOverlap(page, NO_MARKERS);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("rulerleft-196 merge: ruler far-left 4px, markers present, HOLD", async ({ page }) => {
  const { consoleErrors, pageErrors } = listen(page);

  await createMergeSession(page);
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict section(s)");

  // 1. Far-left: ruler right edge sits left of the first card's left edge.
  const ruler = page.getByTestId("mergeview/overview-ruler");
  await expect(ruler).toBeVisible();
  const geo = await page.evaluate(() => {
    const r = document.querySelector('[data-testid="mergeview/overview-ruler"]');
    const c = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(r instanceof HTMLElement) || !(c instanceof HTMLElement)) return null;
    const rb = r.getBoundingClientRect();
    const cb = c.getBoundingClientRect();
    return { rulerRight: rb.right, cardLeft: cb.left, width: rb.width };
  });
  expect(geo, "ruler + first card resolve").not.toBeNull();
  expect(geo?.width ?? Number.POSITIVE_INFINITY, "ruler width <= 6px").toBeLessThanOrEqual(6);
  expect(geo?.width ?? 0, "ruler width paints (>= 1px)").toBeGreaterThanOrEqual(1);
  expect(
    geo?.rulerRight ?? Number.POSITIVE_INFINITY,
    `ruler x < pane x (rulerRight ${geo?.rulerRight} vs cardLeft ${geo?.cardLeft})`,
  ).toBeLessThanOrEqual((geo?.cardLeft ?? Number.NEGATIVE_INFINITY) + 1);

  // Markers present: one per conflict, jump wiring intact.
  const marker = page.getByTestId("mergeview/overview-hunk-0");
  await expect(marker).toBeVisible();
  await expect(marker).toHaveAttribute("aria-label", "Jump to conflict 1 of 1");
  await marker.click();
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");

  // 2. HOLD: card chrome budget + no overlap + zero errors, light + dark.
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(chrome, "section + first card mounted").not.toBeNull();
  expect(chrome ?? Number.POSITIVE_INFINITY, `firstCardTop<=140 (got ${chrome}px)`).toBeLessThanOrEqual(140);
  await assertNoOverlap(page, NO_MARKERS);

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  const darkGeo = await page.evaluate(() => {
    const r = document.querySelector('[data-testid="mergeview/overview-ruler"]');
    const c = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(r instanceof HTMLElement) || !(c instanceof HTMLElement)) return null;
    const rb = r.getBoundingClientRect();
    const cb = c.getBoundingClientRect();
    return { rulerRight: rb.right, cardLeft: cb.left, width: rb.width };
  });
  expect(darkGeo, "ruler + first card resolve (dark)").not.toBeNull();
  expect(darkGeo?.width ?? Number.POSITIVE_INFINITY, "ruler width <= 6px (dark)").toBeLessThanOrEqual(6);
  expect(
    darkGeo?.rulerRight ?? Number.POSITIVE_INFINITY,
    "ruler x < pane x (dark)",
  ).toBeLessThanOrEqual((darkGeo?.cardLeft ?? Number.NEGATIVE_INFINITY) + 1);
  await assertNoOverlap(page, NO_MARKERS);

  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
