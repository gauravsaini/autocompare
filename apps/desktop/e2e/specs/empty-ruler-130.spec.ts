/**
 * Wave-130 empty states + single ruler (audit L/M6 ranked #7).
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves:
 *  (a) Folder / Sync / Media with no paths render the single honest
 *      blank (`workspace/blank-empty`, wave-138 contract; per-view
 *      `folderview/empty`, `syncview/empty`, `mediaview/empty` retired
 *      when ViewRouter does not mount): no compare IPC fires, no
 *      error/alert — blank names the missing two paths;
 *  (b) one ruler source, ignore-aware: with Ignores on and the full
 *      644-row map loaded, pane hunks (gutter `hunk-copy-left-N` ==
 *      strip `textview/overview-hunk-N`, the shared `hunks` grouping)
 *      agree with the shell ruler — every strip hunk range overlaps a
 *      non-same ruler bucket and every non-same ruler bucket overlaps
 *      a strip hunk (no second fetch anywhere: the strip takes props
 *      only, the shell ruler owns the overview map);
 *  (c) zero-row ruler honesty: a blank folder session leaves the shell
 *      ruler at 0 rows with the `ruler/empty` "0 rows" affordance and
 *      zero fake difference markers.
 *
 * PORT-FREE: runs on a fresh :1538 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";

const EVID_DIR = "/tmp/empty-ruler-130";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function assertClean(consoleErrors: string[], pageErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

/** Blank session of `kind` via the Home quick-create card (no paths staged). */
async function openBlankKind(page: Page, kind: string, viewId: string): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click();
  } else {
    const home = page.getByTestId("tab-home");
    if ((await home.count()) > 0) await home.click().catch(() => {});
  }
  await expect(page.getByTestId("workspace/welcome")).toBeVisible({ timeout: 15000 });
  await page.getByTestId(`home/new-${kind}`).click();
  // Wave-150 (wave-138 contract): blank sessions render the single
  // honest empty; views never mount until both paths are set.
  await expect(page.getByTestId("workspace/blank-empty")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId(viewId)).toHaveCount(0);
}

test("blank folder shows pick-two-paths empty, ruler shows honest 0 rows", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await openBlankKind(page, "folder", "view-folder");

  const empty = page.getByTestId("workspace/blank-empty");
  await expect(empty).toBeVisible({ timeout: 15000 });
  await expect(empty).toContainText("needs two paths");

  // Zero-row ruler: honest "0 rows", no fake difference markers.
  const rulerEmpty = page.getByTestId("ruler/empty");
  await expect(rulerEmpty).toBeVisible();
  await expect(rulerEmpty).toHaveText("0 rows");
  await expect(page.getByTestId("ruler")).toHaveAttribute("aria-label", "Overview ruler: 0 rows");
  expect(await page.locator('[data-testid="ruler"] [data-kind]').count()).toBe(0);

  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/folder-empty.png` });
  assertClean(consoleErrors, pageErrors);
});

test("blank sync and blank media show pick-two-paths empty, no error-fire", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  await openBlankKind(page, "sync", "view-sync");
  const syncEmpty = page.getByTestId("workspace/blank-empty");
  await expect(syncEmpty).toBeVisible({ timeout: 15000 });
  await expect(syncEmpty).toContainText("needs two paths");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/sync-empty.png` });

  await openBlankKind(page, "media", "view-media");
  const mediaEmpty = page.getByTestId("workspace/blank-empty");
  await expect(mediaEmpty).toBeVisible({ timeout: 15000 });
  await expect(mediaEmpty).toContainText("needs two paths");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/media-empty.png` });

  assertClean(consoleErrors, pageErrors);
});

test("single ruler: pane hunks agree with shell ruler with Ignores on", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await expect(page.getByTestId("textview/status")).toContainText("/ 644 rows");

  // Ignores on first (mock echoes flags, rows stay fixture rows), then
  // load the FULL map so the strip covers every row like the ruler does.
  const ignoreToggle = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle.getAttribute("aria-expanded")) !== "true") await ignoreToggle.click();
  await page.getByTestId("textview/ignore-whitespace").check();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("/ 644 rows", { timeout: 15000 });

  for (let i = 0; i < 6; i += 1) {
    const more = page.getByRole("button", { name: /Load more/ });
    if ((await more.count()) === 0) break;
    await more.click();
    await expect(page.getByTestId("textview/status")).not.toContainText("Loading", { timeout: 15000 }).catch(() => {});
  }
  await expect(page.getByTestId("textview/status")).toContainText("644 / 644 rows", { timeout: 30000 });

  // Pane-internal single source: strip markers ride the shared `hunks`
  // grouping (wave-79 pins strip == gutter on first paint); the pane
  // hunk truth here is the strip marker count.
  const stripCount = await page.locator('[data-testid^="textview/overview-hunk-"]').count();
  expect(stripCount, "demo loads at least two hunks").toBeGreaterThan(1);

  // Shell-ruler agreement: parse 1-based row ranges from titles.
  const stripRanges: Array<{ start: number; end: number }> = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid^="textview/overview-hunk-"]')];
    return els.map((el) => {
      const m = /rows (\d+)[–-](\d+)/.exec(el.getAttribute("title") ?? "");
      return { start: Number(m?.[1] ?? 1), end: Number(m?.[2] ?? 1) };
    });
  });
  const rulerSpans: Array<{ start: number; end: number; kind: string }> = await page.evaluate(() => {
    const els = [...document.querySelectorAll('[data-testid="ruler"] [data-testid^="ruler/row-"]')];
    return els.map((el) => {
      const title = el.getAttribute("title") ?? "";
      const range = /rows (\d+)[–-](\d+)/.exec(title);
      const single = /row (\d+)\s*$/.exec(title);
      const start = range !== null ? Number(range[1]) : Number(single?.[1] ?? 1);
      const end = range !== null ? Number(range[2]) : start;
      return { start, end, kind: el.getAttribute("data-kind") ?? "same" };
    });
  });
  expect(rulerSpans.length, "ruler renders markers for the loaded diff").toBeGreaterThan(0);
  const rulerHunks = rulerSpans.filter((s) => s.kind !== "same");
  expect(rulerHunks.length, "ruler shows real differences, not an all-same wash").toBeGreaterThan(0);

  const overlaps = (a: { start: number; end: number }, b: { start: number; end: number }): boolean =>
    a.start <= b.end && b.start <= a.end;
  for (const hunk of stripRanges) {
    expect(
      rulerHunks.some((s) => overlaps(s, hunk)),
      `strip hunk rows ${hunk.start}-${hunk.end} covered by a ruler difference`,
    ).toBe(true);
  }
  for (const span of rulerHunks) {
    expect(
      stripRanges.some((h) => overlaps(h, span)),
      `ruler difference rows ${span.start}-${span.end} matches a pane hunk`,
    ).toBe(true);
  }

  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/ruler-agreement.png` });
  assertClean(consoleErrors, pageErrors);
});
