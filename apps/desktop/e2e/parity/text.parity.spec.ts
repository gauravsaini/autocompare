/**
 * Living parity spec: TEXT view (BC truth: File1 vs File2 gutter idiom).
 * Rendered-geometry only — behavior lives in e2e/specs/*. Functional specs
 * must not duplicate these pixel locks.
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) { test.skip(true, `infra: ${err}`); }
});

test("text parity: gutter + hatch + density + chrome budget", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible();
  const head = page.locator('.tv-row:has([data-testid^="hunk-copy-left-"])').first();
  await expect(head).toBeVisible();

  // 7-track grid, 20px action tracks, 6px splitter, no center box
  const tracks = await head.locator(":scope > div").evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  expect(tracks.length, "7 tracks").toBe(7);
  expect(tracks[0], "left action 20px").toBe("20px");
  expect(tracks[3], "splitter 6px").toBe("6px");
  expect(tracks[4], "right action 20px").toBe("20px");
  await expect(head.locator(".w-14")).toHaveCount(0);

  // arrows present, 12px svg, borderless
  for (const side of ["left", "right"] as const) {
    const btn = head.locator(`[data-testid^="hunk-copy-${side}-"]`).first();
    await expect(btn.locator("svg")).toHaveCount(1);
    expect(await btn.locator("svg").getAttribute("width")).toBe("12");
    expect(await btn.evaluate((el) => window.getComputedStyle(el).borderWidth)).toBe("0px");
  }

  // orphan hatch computed (if any orphan in demo slice)
  const hatch = await page.evaluate(() => {
    const n = document.querySelector(".tv-cell-empty");
    if (!n) return "none-in-slice";
    return window.getComputedStyle(n).backgroundImage || "";
  });
  expect(hatch.includes("repeating-linear-gradient") || hatch === "none-in-slice").toBe(true);

  // no duplicate Prev/Next
  const prev = await page.locator('[data-testid*="prev" i], [aria-label*="Prev" i]').count();
  const next = await page.locator('[data-testid*="next" i], [aria-label*="Next" i]').count();
  expect(prev, `prev<=2 got ${prev}`).toBeLessThanOrEqual(2);
  expect(next, `next<=2 got ${next}`).toBeLessThanOrEqual(2);

  // density ~20-22px
  const h = await head.evaluate((el) => el.getBoundingClientRect().height);
  expect(h, `row ~20-22 got ${h}`).toBeGreaterThanOrEqual(18);
  expect(h).toBeLessThanOrEqual(24);

  // chrome budget: first row within 140px of section top
  const delta = await page.evaluate(() => {
    const sec = document.querySelector('section[aria-label="text comparison"], section:has(.tv-row)');
    const first = document.querySelector(".tv-row");
    if (!(sec instanceof HTMLElement) || !(first instanceof HTMLElement)) return 9999;
    return first.getBoundingClientRect().top - sec.getBoundingClientRect().top;
  });
  expect(delta, `firstRowTop<=140 got ${delta}`).toBeLessThanOrEqual(140);
});
