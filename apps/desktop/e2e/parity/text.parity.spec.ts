/**
 * Living parity spec: TEXT view (BC truth: File1 vs File2 gutter idiom).
 * Rendered-geometry only — behavior lives in e2e/specs/*. Functional specs
 * must not duplicate these pixel locks (no offsetHeight/gridTemplateColumns
 * pins outside this file).
 *
 * Run: fresh port, single worker, mock IPC —
 *   npx playwright test --config ../../.tmp-waves/playwright.fresh.config.mjs \
 *     e2e/parity/text.parity.spec.ts --workers=1
 * (VITE_MOCK_IPC=1 is set by the fresh config's webServer env.)
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) { test.skip(true, `infra: ${err}`); }
});

test("text parity: gutter + splitter + hatch + bracket + density + chrome budget", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible();
  const head = page.locator('.tv-row:has([data-testid^="hunk-copy-left-"])').first();
  await expect(head).toBeVisible();

  const metrics: Record<string, string | number> = {};

  // 1. Gutter 20px: 7-track grid, per-pane action tracks 20px.
  const tracks = await head.locator(":scope > div").evaluate((el) => window.getComputedStyle(el).gridTemplateColumns.split(" "));
  metrics.tracks = tracks.join(" ");
  expect(tracks.length, "7 tracks").toBe(7);
  expect(tracks[0], "left action gutter 20px").toBe("20px");
  expect(tracks[4], "right action gutter 20px").toBe("20px");

  // 2. Splitter 6px: center track 6px with centered 1px rail.
  expect(tracks[3], "splitter 6px").toBe("6px");
  const splitterW = await head.locator(".tv-splitter").evaluate((el) => el.getBoundingClientRect().width);
  metrics.splitterW = Math.round(splitterW * 100) / 100;
  expect(splitterW, `splitter ~6px got ${splitterW}`).toBeGreaterThan(5);
  expect(splitterW, `splitter ~6px got ${splitterW}`).toBeLessThan(7);
  const splitterRail = await head.locator(".tv-splitter").evaluate((el) => {
    const cs = window.getComputedStyle(el, "::before");
    return { width: cs.width, content: cs.content };
  });
  expect(splitterRail.content, "splitter paints centered ::before rail").not.toBe("none");
  expect(splitterRail.width, "splitter rail 1px").toBe("1px");

  // 3. No 56px center box: no .w-14 in row, no 56px track.
  await expect(head.locator(".w-14"), "no 56px center box").toHaveCount(0);
  expect(tracks, "no 56px track").not.toContain("56px");

  // 4. Arrows: per-pane take arrows, 12px svg, borderless, amber ink.
  for (const side of ["left", "right"] as const) {
    const btn = head.locator(`[data-testid^="hunk-copy-${side}-"]`).first();
    await expect(btn.locator("svg")).toHaveCount(1);
    expect(await btn.locator("svg").getAttribute("width")).toBe("12");
    expect(await btn.evaluate((el) => window.getComputedStyle(el).borderWidth)).toBe("0px");
  }

  // 5. Bracket rail: hunk roles on BOTH per-pane gutters; multi-line hunks
  // paint a 1px ::before stem, single-row hunks are arrow-only (no rail).
  const brackets = await head.locator("span[data-bracket]").evaluateAll((els) => els.map((el) => el.getAttribute("data-bracket")));
  metrics.bracketRoles = brackets.join(",");
  expect(brackets.length, "both per-pane gutters carry bracket roles").toBe(2);
  expect(["start", "middle", "end", "single"], `bracket roles known, got ${brackets}`).toContain(brackets[0]);
  const railW = await head.locator("span[data-bracket]").first().evaluate((el) => window.getComputedStyle(el, "::before").width);
  metrics.railW = railW;
  if (brackets[0] === "single") {
    const content = await head.locator("span[data-bracket]").first().evaluate((el) => window.getComputedStyle(el, "::before").content);
    expect(content, "single hunk mounts no rail").toBe("none");
  } else {
    expect(railW, `bracket rail 1px stem, got ${railW}`).toBe("1px");
  }

  // 6. Hatch on empty: orphan null cell paints pure-CSS diagonal hatch,
  // untinted, zero extra DOM; live sibling keeps its tint.
  const emptyCell = head.locator(".tv-cell-empty").first();
  await expect(emptyCell, "orphan empty cell exists in hunk head").toHaveCount(1);
  expect(await emptyCell.evaluate((el) => el.childElementCount), "zero extra DOM in null cell").toBe(0);
  for (const tint of ["tv-del", "tv-ins", "tv-chg"]) {
    await expect(emptyCell).not.toHaveClass(new RegExp(tint));
  }
  const hatch = await emptyCell.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { image: cs.backgroundImage, bg: cs.backgroundColor };
  });
  metrics.hatchImage = hatch.image.slice(0, 80);
  metrics.hatchBg = hatch.bg;
  expect(hatch.image, "orphan hatch is pure-CSS repeating gradient").toMatch(/repeating-linear-gradient|linear-gradient|-45deg/);

  // no duplicate Prev/Next diff nav
  const prev = await page.locator('[data-testid*="prev" i], [aria-label*="Prev" i]').count();
  const next = await page.locator('[data-testid*="next" i], [aria-label*="Next" i]').count();
  expect(prev, `prev<=2 got ${prev}`).toBeLessThanOrEqual(2);
  expect(next, `next<=2 got ${next}`).toBeLessThanOrEqual(2);

  // 7. Density: row 20-22px (rect + offsetHeight both in range).
  const h = await head.evaluate((el) => el.getBoundingClientRect().height);
  const offsetH = await head.evaluate((el) => (el as HTMLElement).offsetHeight);
  metrics.rowRectH = Math.round(h * 100) / 100;
  metrics.rowOffsetH = offsetH;
  expect(h, `row 20-22px, got ${h}`).toBeGreaterThanOrEqual(20);
  expect(h, `row 20-22px, got ${h}`).toBeLessThanOrEqual(22);
  expect(offsetH, `row offsetHeight 20-22px, got ${offsetH}`).toBeGreaterThanOrEqual(20);
  expect(offsetH, `row offsetHeight 20-22px, got ${offsetH}`).toBeLessThanOrEqual(22);

  // 8. Chrome budget: first row within 140px of section top.
  const delta = await page.evaluate(() => {
    const sec = document.querySelector('section[aria-label="text comparison"], section:has(.tv-row)');
    const first = document.querySelector(".tv-row");
    if (!(sec instanceof HTMLElement) || !(first instanceof HTMLElement)) return 9999;
    return first.getBoundingClientRect().top - sec.getBoundingClientRect().top;
  });
  metrics.firstRowTop = Math.round(delta * 100) / 100;
  expect(delta, `firstRowTop<=140 got ${delta}`).toBeLessThanOrEqual(140);

  // Evidence: Ours screenshot of the text section for the [BC|Ours] pair.
  const section = page.locator('section[aria-label="text comparison"], section:has(.tv-row)').first();
  await section.screenshot({ path: "test-results/parity-text-ours.png" }).catch(() => {});
  await page.screenshot({ path: "test-results/parity-text-ours-full.png" }).catch(() => {});

  console.log(`PARITY-METRICS ${JSON.stringify(metrics)}`);
});
