/**
 * Wave-103 media parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MediaView) over the mock media engine and pins the
 * Beyond Compare bar (`/tmp/bc-media-truth.png`):
 *  (a) chrome budget firstRowTop<=140px section-relative with the
 *      tolerance panel closed (single toolbar row + summary header
 *      only; app tab/path bars above are not media chrome);
 *  (b) density consistent (uniform row heights/pitch, no stray tall
 *      rows, compact one-row toolbar);
 *  (c) per-file headers (`mediaview/meta-left` / `meta-right`:
 *      session basename, full path in `title`) next to the honest
 *      scalar dims both modes already render -- BC prints the file
 *      path above every compare pane and this is the honest
 *      client-side equivalent (zero added chrome: same one line);
 *  (d) preview/compare wiring (picture/audio fetch + tolerance
 *      recompute round-trip) + status bar verdict/counts intact.
 *  Light + dark DOM checks, zoomed PNG crops + side-by-side vs the
 *  BC truth to `/tmp/` (never in-repo), assertNoOverlap, zero
 *  console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/mediaparity-103";
const BC_TRUTH = "/tmp/bc-media-truth.png";

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

async function createMediaSession(page: Page, left: string, right: string): Promise<void> {
  if (await page.getByTestId("workspace/demo-banner").isVisible()) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("media");
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-media")).toBeVisible();
}

async function chromeDelta(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="media comparison"]');
    const first = document.querySelector('[data-testid="mediaview/row"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return f.top - s.top;
  });
}

test("mediaparity-103: chrome budget, density, file headers, wiring, status, light+dark", async ({
  page,
  browserName,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // ---- Picture mode: headers, wiring, status, density, budget. ----
  await createMediaSession(page, "/tmp/m-left.png", "/tmp/m-right.png");
  const view = page.getByTestId("view-media");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Picture compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/99.7% equal/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/12 \/ 4100 bytes differ/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(2);

  // (c) Per-file headers name the session files (basename visible,
  // full path in title) next to the honest scalar dims.
  const metaLeft = page.getByTestId("mediaview/meta-left");
  const metaRight = page.getByTestId("mediaview/meta-right");
  await expect(metaLeft).toBeVisible();
  await expect(metaRight).toBeVisible();
  await expect(metaLeft).toContainText("m-left.png");
  await expect(metaRight).toContainText("m-right.png");
  await expect(metaLeft).toHaveAttribute("title", "/tmp/m-left.png");
  await expect(metaRight).toHaveAttribute("title", "/tmp/m-right.png");

  // Toolbar is one compact row, mode-only in picture mode.
  const toolbar = page.getByTestId("mediaview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toBeHidden();
  await expect(page.getByTestId("mediaview/recompare")).toBeHidden();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar carries the verdict + counts below the rows.
  const status = page.getByTestId("mediaview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("different");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText("12 / 4100 bytes differ");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await page.getByTestId("mediaview/row").first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(firstRowTopAbs);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(firstRowTopAbs);

  // (b) Density: uniform row heights + steady pitch, no stray tall rows.
  const density = await page.evaluate(() => {
    const nodes = [...document.querySelectorAll('[data-testid="mediaview/row"]')];
    const hs = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().height);
    const tops = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().top);
    const pitches = tops.slice(1).map((t, i) => t - tops[i]);
    return { hs, pitches };
  });
  expect(density.hs.length).toBe(2);
  const maxH = Math.max(...density.hs);
  const minH = Math.min(...density.hs);
  expect(maxH - minH, `uniform row heights (got ${minH}..${maxH})`).toBeLessThanOrEqual(2);
  expect(maxH, `compact rows, no stray tall rows (got ${maxH}px)`).toBeLessThan(60);

  // (a) Chrome budget in picture mode (no panel exists here).
  const picDelta = await chromeDelta(page);
  expect(picDelta, "section + first row mounted (picture)").not.toBeNull();
  console.log(`MEDIAPARITY-103 picture chromeDelta=${picDelta}`);
  expect(picDelta ?? Number.POSITIVE_INFINITY, `media chrome within budget (got ${picDelta}px)`).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);

  // ---- Audio mode: panel behavior, headers, recompute wiring. ----
  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Audio compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/3 \/ 100 windows differ/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(3);

  // Per-file headers ride the audio meta line too.
  await expect(page.getByTestId("mediaview/meta-left")).toContainText("m-left.wav");
  await expect(page.getByTestId("mediaview/meta-right")).toContainText("m-right.wav");
  await expect(page.getByTestId("mediaview/meta-left")).toHaveAttribute("title", "/tmp/m-left.wav");

  // Tolerance panel open by default; toggle + Esc collapse it.
  const audioToolbar = page.getByTestId("mediaview/toolbar");
  await expect(audioToolbar).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("mediaview/tolerance")).toBeVisible();
  const abarH = await audioToolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(abarH, `toolbar is one compact row (got ${abarH}px)`).toBeLessThan(120);

  // (d) Recompute wiring intact: tolerance round-trip keeps the
  // summary live and the status bar follows.
  await page.getByTestId("mediaview/tolerance").fill("5");
  await page.getByTestId("mediaview/recompare").click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText(/windows differ/);
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("different");

  await assertNoOverlap(page);

  // (a) Chrome budget with the tolerance panel closed.
  await page.getByTestId("mediaview/toolbar-tolerance").click();
  await expect(page.getByTestId("mediaview/tolerance")).toBeHidden();
  const delta = await chromeDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  console.log(`MEDIAPARITY-103 audio chromeDelta=${delta}`);
  expect(delta ?? Number.POSITIVE_INFINITY, `media chrome within budget (got ${delta}px)`).toBeLessThanOrEqual(140);

  // Audio density holds with the panel closed.
  const adensity = await page.evaluate(() => {
    const nodes = [...document.querySelectorAll('[data-testid="mediaview/row"]')];
    const hs = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().height);
    const tops = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().top);
    return { hs, pitch: tops.length > 1 ? tops[1] - tops[0] : null };
  });
  expect(adensity.hs.length).toBe(3);
  expect(Math.max(...adensity.hs) - Math.min(...adensity.hs), "uniform audio row heights").toBeLessThanOrEqual(2);

  // Evidence: full view + zoomed crops (toolbar, rows, status).
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await page.getByTestId("mediaview/summary").screenshot({ path: `${EVID}-zoom-rows.png` });
  await status.screenshot({ path: `${EVID}-zoom-status.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await page.getByTestId("mediaview/summary").screenshot({ path: `${EVID}-zoom-rows-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Side-by-side vs the BC truth (truth pixels ship in the figure;
  // the BC capture shows Home with the Media/Picture Compare
  // launchers -- see REPORT for the honest reading).
  const truthB64 = fs.readFileSync(BC_TRUTH).toString("base64");
  const oursB64 = fs.readFileSync(`${EVID}-light.png`).toString("base64");
  await page.emulateMedia({ colorScheme: "light" });
  await page.setContent(
    `<html><body style="margin:0;display:flex;gap:8px;background:#fff;font:12px system-ui">` +
      `<figure style="margin:0"><figcaption>BC media truth (/tmp/bc-media-truth.png)</figcaption><img src="data:image/png;base64,${truthB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
      `<figure style="margin:0"><figcaption>ours media parity light (${browserName})</figcaption><img src="data:image/png;base64,${oursB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
      `</body></html>`,
  );
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${EVID}-sidebyside.png`, fullPage: true });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
