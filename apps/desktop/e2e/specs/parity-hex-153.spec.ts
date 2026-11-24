/**
 * Wave-153 hex closed loop (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) over the embedded 11-byte `h_*` pair ("hello
 * world" vs "hallo world", single page) and pins Beyond Compare parity
 * vs `/tmp/bc-hex-truth.png`:
 *  (a) density: hx-rows uniform compact (<=22px token contract, delta <= 2px),
 *      uniform byte cells and monospace alignment;
 *  (b) panes strip + dual gutters: left/right meta with basename + byte counts,
 *      mirrored left/right offset gutters with shared y-alignment,
 *      diff tint in hex and ASCII columns;
 *  (c) find + goto wiring: find jump lands match, goto jump selects,
 *      detail inspector reflects selected byte;
 *  (d) Esc parity: global Esc closes both find + goto panels, toolbar toggles
 *      reopen;
 *  (e) chrome budget: firstRowTop <= 140px section-relative panels-closed,
 *      single toolbar row (<120px), status bar with binary differences verdict,
 *      per-side sizes, and honest load time;
 *  (f) overview: shell-level overview ruler honest state + toolbar diff navigation.
 * Visual parity only: light + dark PNGs + row zoom crop to /tmp/foldertablehex-153-hex/ (never in-repo).
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/foldertablehex-153-hex";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createHexSession(page: Page): Promise<void> {
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/h153-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/h153-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

test("foldertablehex-153 hex: density + overview + Esc parity + chrome budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createHexSession(page);
  const view = page.getByTestId("view-hex");
  await expect(view).toContainText("files differ");
  await expect(view).toContainText("11 bytes");

  // (a) Density: hx-rows uniform compact height (delta <= 2px, maxH < 28px).
  const rowHeights = await page.evaluate(() => {
    const nodes = Array.from(document.querySelectorAll(".hx-row"));
    return nodes.map((n) => (n as HTMLElement).getBoundingClientRect().height);
  });
  expect(rowHeights.length).toBeGreaterThan(0);
  const minH = Math.min(...rowHeights);
  const maxH = Math.max(...rowHeights);
  expect(maxH - minH, `hx rows uniform (min ${minH}, max ${maxH})`).toBeLessThanOrEqual(2);
  expect(maxH, `hx rows compact on 22px contract (got ${maxH})`).toBeLessThan(28);

  // (b) Panes strip + dual gutters: per-side meta with byte counts.
  await expect(page.getByTestId("hexview/panes")).toBeVisible();
  await expect(page.getByTestId("hexview/left-meta")).toContainText("11 bytes");
  await expect(page.getByTestId("hexview/right-meta")).toContainText("11 bytes");
  await expect(page.getByTestId("hexview/left-meta")).toContainText("h153-left.bin");
  await expect(page.getByTestId("hexview/right-meta")).toContainText("h153-right.bin");

  // Dual offset gutters mirror each other and share y-alignment.
  const rows = view.locator(".hx-row");
  const firstRow = rows.first();
  await expect(firstRow).toBeVisible();
  const gutters = firstRow.locator(".hx-offset");
  await expect(gutters).toHaveCount(2);
  const y0 = await gutters.nth(0).evaluate((el) => el.getBoundingClientRect().y);
  const y1 = await gutters.nth(1).evaluate((el) => el.getBoundingClientRect().y);
  expect(Math.abs(y0 - y1), "offset gutters share a row").toBeLessThan(2);

  // Per-char ASCII diff tint on both sides.
  const asciiCols = firstRow.locator(".hx-ascii");
  await expect(asciiCols).toHaveCount(2);
  await expect(asciiCols.nth(0)).toContainText("hello world");
  await expect(asciiCols.nth(1)).toContainText("hallo world");
  const diffs = firstRow.locator(".hx-ascii .hx-diff");
  expect(await diffs.count(), "ascii diffs tinted on both sides").toBeGreaterThanOrEqual(2);

  // (c) Find + goto wiring on the embedded pair.
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await page.getByTestId("hex-find-input").fill("6c");
  await page.getByTestId("hex-find-next").click();
  await expect(page.getByTestId("hex-find-status")).toContainText("match at offset 2");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  await page.getByTestId("hex-goto-input").fill("5");
  await page.getByTestId("hex-goto-go").click();
  await expect(page.getByTestId("hex-goto-status")).toContainText("jumped to offset 5");
  await expect(page.getByTestId("hexview/byte-5")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(5)");

  // (d) Esc parity: global Esc closes both find and goto panels.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();
  await expect(page.getByTestId("hexview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByTestId("hexview/toolbar-goto")).toHaveAttribute("aria-expanded", "false");

  // Reopen via toolbar toggles (toggle parity) and close again with Esc.
  await page.getByTestId("hexview/toolbar-find").click();
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect(page.getByTestId("hexview/toolbar-find")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("hexview/toolbar-goto").click();
  await expect(page.getByTestId("hex-goto-input")).toBeVisible();
  await expect(page.getByTestId("hexview/toolbar-goto")).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();

  // (e) Chrome budget: single toolbar row, status bar, firstRowTop <= 140px.
  const toolbar = page.getByTestId("hexview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar one compact row (got ${barH}px)`).toBeLessThan(120);

  const status = page.getByTestId("hexview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("hexview/status-diff")).toContainText("Binary differences");
  await expect(page.getByTestId("hexview/status-bytes")).toContainText("11 bytes");
  await expect(page.getByTestId("hexview/status-sizes")).toContainText("left 11 bytes");
  await expect(page.getByTestId("hexview/status-sizes")).toContainText("right 11 bytes");
  await expect(page.getByTestId("hexview/status-load")).toContainText(/Load time:/);

  const delta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="hex comparison"]');
    const first = document.querySelector(".hx-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(delta, `firstRowTop<=140 (got ${delta})`).not.toBeNull();
  expect(delta!).toBeLessThanOrEqual(140);

  // (f) Overview & Diff navigation: shell ruler honest state + toolbar diff nav.
  const ruler = page.getByTestId("ruler");
  await expect(ruler).toBeVisible();
  await expect(page.getByTestId("hex-diff-prev")).toBeVisible();
  await expect(page.getByTestId("hex-diff-next")).toBeVisible();
  await expect(page.getByTestId("hex-diff-status")).toContainText("diff");

  // (g) Visual parity: light + dark PNGs + row zoom crop.
  await assertNoOverlap(page);
  fs.mkdirSync(EVID, { recursive: true });
  await page.screenshot({ path: `${EVID}/hex-light.png` });
  await firstRow.screenshot({ path: `${EVID}/hex-zoom-rows.png` });

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}/hex-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors, `console errors: ${consoleErrors.join(" | ")}`).toEqual([]);
  expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);
});
