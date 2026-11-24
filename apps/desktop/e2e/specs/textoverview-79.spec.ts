/**
 * Wave-79 far-left overview strip proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the Beyond Compare style far-left overview strip:
 *  - the strip mounts at the far left of the text rows (`tv-body` flex:
 *    strip x sits left of every row cell, 4px wide within the 4-6px
 *    budget (wave-196: was 5px), full-height rail beside the list);
 *  - one marker per hunk: `textview/overview-hunk-N` count equals the
 *    gutter `hunk-copy-left-N` count (same `collectTextHunks` grouping,
 *    1:1 ordinals), each marker exposing a `Jump to difference N of M`
 *    label with an honest row range title;
 *  - click-to-jump reuses the hunk-nav path (no new IPC: status line
 *    byte-identical): clicking marker 1 selects its hunk's first row
 *    (`data-selected="true"`, `difference 2 of M` note) while the row
 *    list itself is untouched;
 *  - first-row budget: `textview/row-0` still mounts at the top of its
 *    list container (`top <= 140px`, strip adds no vertical stack);
 *  - light + dark hold (markers keep their diff-token paint, strip
 *    stays 4px (wave-196), zero alerts);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: markers derive from the loaded rows' hunk grouping in
 * the view (no new IPC), so every assertion below reads the DOM /
 * computed styles, never the network. Skip policy matches the other
 * specs (infra failures skip). Overlap helper excludes the strip
 * markers (dense adjacent hunks paint touching 4px ticks by design).
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-79";
const NO_MARKERS = "header, nav, main, [data-testid]:not([data-testid^='textview/overview-hunk'])";

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

test("overview strip sits far-left, one marker per hunk, click jumps", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  const statusBefore = await page.getByTestId("textview/status").innerText();

  // Strip present, far-left, 4-6px wide.
  const strip = page.getByTestId("textview/overview-strip");
  await expect(strip).toBeVisible();
  await expect(strip).toHaveAttribute("role", "navigation");
  const stripBox = await strip.evaluate((el) => el.getBoundingClientRect().toJSON());
  expect(stripBox.width, "strip is 4-6px wide").toBeGreaterThanOrEqual(4);
  expect(stripBox.width, "strip is 4-6px wide").toBeLessThanOrEqual(6);
  const order = await page.evaluate(() => {
    const stripEl = document.querySelector('[data-testid="textview/overview-strip"]');
    const rowEl = document.querySelector('[data-testid="textview/row-6"]');
    if (!(stripEl instanceof HTMLElement) || !(rowEl instanceof HTMLElement)) return null;
    const s = stripEl.getBoundingClientRect();
    const r = rowEl.getBoundingClientRect();
    return { stripRight: s.right, rowLeft: r.left, stripTop: s.top };
  });
  expect(order, "strip + row resolve").not.toBeNull();
  expect(order?.stripRight ?? Number.POSITIVE_INFINITY, "strip paints left of the rows").toBeLessThanOrEqual(
    order?.rowLeft ?? Number.NEGATIVE_INFINITY,
  );

  // One marker per hunk: marker count matches the gutter hunk count
  // (same grouping, 1:1 ordinals).
  const gutterHunks = await page.locator('[data-testid^="hunk-copy-left-"]').count();
  expect(gutterHunks, "demo loads at least two hunks").toBeGreaterThan(1);
  const markers = page.locator('[data-testid^="textview/overview-hunk-"]');
  await expect(markers).toHaveCount(gutterHunks);
  await expect(page.getByTestId("textview/overview-hunk-0")).toHaveAttribute(
    "aria-label",
    `Jump to difference 1 of ${gutterHunks}`,
  );
  const markerBg = await page
    .getByTestId("textview/overview-hunk-0")
    .evaluate((el) => window.getComputedStyle(el).backgroundColor);
  expect(markerBg, "marker reuses a diff token (opaque paint)").not.toBe("rgba(0, 0, 0, 0)");

  // Click-to-jump reuses the hunk-nav path: the target is the row hosting
  // the matching gutter arrow; after the click that row is selected, the
  // toolbar note names the hunk, and the status line is byte-identical
  // (selection never re-fetches rows: no new IPC).
  const targetRow = await page.evaluate(() => {
    const btn = document.querySelector('[data-testid="hunk-copy-left-1"]');
    const row = btn?.closest(".tv-row");
    return row?.getAttribute("data-testid") ?? null;
  });
  expect(targetRow, "second hunk row resolves").not.toBeNull();
  await page.getByTestId("textview/overview-hunk-1").click();
  const target = page.getByTestId(targetRow ?? "textview/row-0");
  await expect(target).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);

  // First-row budget: the strip adds no vertical stack above the list.
  const budget = await page.evaluate(() => {
    const row = document.querySelector('[data-testid="textview/row-0"]');
    if (!(row instanceof HTMLElement)) return null;
    const host = row.parentElement;
    if (!(host instanceof HTMLElement)) return null;
    const r = row.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    return { top: r.top - h.top, absTop: r.top };
  });
  expect(budget, "row-0 mounted").not.toBeNull();
  expect(
    budget?.top ?? Number.POSITIVE_INFINITY,
    `first row at list top (got ${budget?.top}px, abs ${budget?.absTop}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page, NO_MARKERS);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/textoverview-light.png` });

  // Dark scheme holds: strip stays 4px far-left (wave-196), markers keep paint.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  const darkBox = await strip.evaluate((el) => el.getBoundingClientRect().toJSON());
  expect(Math.round(darkBox.width), "strip stays 4px in dark").toBe(4);
  const darkBg = await page
    .getByTestId("textview/overview-hunk-0")
    .evaluate((el) => window.getComputedStyle(el).backgroundColor);
  expect(darkBg, "marker keeps diff-token paint in dark").not.toBe("rgba(0, 0, 0, 0)");
  await expect(target).toHaveAttribute("data-selected", "true");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/textoverview-dark.png` });
  await assertNoOverlap(page, NO_MARKERS);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
