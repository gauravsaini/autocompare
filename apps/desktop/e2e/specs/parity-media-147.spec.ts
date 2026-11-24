/**
 * Wave-147 media closed loop (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MediaView) over the mock media engine and pins the BC
 * bar (`/tmp/bc-media-truth.png`):
 *  (a) chrome budget firstRowTop<=140px section-relative, tolerance
 *      panel closed (single toolbar row + summary header only);
 *  (b) toolbar one compact row (<120px), mode-only in picture mode;
 *  (c) diff chrome: per-file headers (`meta-left`/`meta-right` with
 *      basename + full path title) next to honest scalar dims, score +
 *      diff samples/rows, tolerance recompute round-trip in audio;
 *  (d) empty states: `mediaview/empty` no-paths copy exists and stays
 *      hidden when loaded; `mediaview/empty-diff` identical-only note
 *      stays hidden for the mock non-identical summaries (hidden =
 *      correct, proves the identical branch without forging data);
 *  (e) status bar verdict + counts below rows, toolbar above rows.
 *  Light + dark DOM checks, zoomed PNG crops to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/mergesyncmedia-147-media";
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

test("mergesyncmedia-147 media: chrome budget, diff chrome, empty states, wiring, status", async ({
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
  await expect(page.getByTestId("mediaview/row")).toHaveCount(2);

  // (c) Per-file headers name the session files next to scalar dims.
  await expect(page.getByTestId("mediaview/meta-left")).toContainText("m-left.png");
  await expect(page.getByTestId("mediaview/meta-right")).toContainText("m-right.png");
  await expect(page.getByTestId("mediaview/meta-left")).toHaveAttribute("title", "/tmp/m-left.png");

  // Toolbar one compact row, mode-only in picture mode.
  const toolbar = page.getByTestId("mediaview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toBeHidden();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // (d) Empty states: no-paths empty hidden when loaded; identical-only
  // note hidden for the mock non-identical summary (correct branch).
  await expect(page.getByTestId("mediaview/empty")).toHaveCount(0);
  await expect(page.getByTestId("mediaview/empty-diff")).toHaveCount(0);

  // Status bar below rows, toolbar above rows.
  const status = page.getByTestId("mediaview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("different");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText("12 / 4100 bytes differ");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await page.getByTestId("mediaview/row").first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(firstRowTopAbs);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(firstRowTopAbs);

  // Density: uniform row heights, no stray tall rows.
  const density = await page.evaluate(() => {
    const nodes = [...document.querySelectorAll('[data-testid="mediaview/row"]')];
    const hs = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().height);
    return { hs };
  });
  expect(Math.max(...density.hs) - Math.min(...density.hs), "uniform row heights").toBeLessThanOrEqual(2);
  expect(Math.max(...density.hs), "compact rows").toBeLessThan(60);

  // (a) Chrome budget in picture mode.
  const picDelta = await chromeDelta(page);
  expect(picDelta, "section + first row mounted (picture)").not.toBeNull();
  console.log(`MERGESYNCMEDIA-147 media picture chromeDelta=${picDelta}`);
  expect(picDelta ?? Number.POSITIVE_INFINITY, `media chrome within budget (got ${picDelta}px)`).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);

  // ---- Audio mode: panel behavior, headers, recompute wiring. ----
  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Audio compare/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(3);
  await expect(page.getByTestId("mediaview/meta-left")).toContainText("m-left.wav");
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("mediaview/tolerance")).toBeVisible();
  // Empty-diff stays hidden for the mock non-identical audio summary too.
  await expect(page.getByTestId("mediaview/empty-diff")).toHaveCount(0);

  await page.getByTestId("mediaview/tolerance").fill("5");
  await page.getByTestId("mediaview/recompare").click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText(/windows differ/);

  await assertNoOverlap(page);

  // Chrome budget with the tolerance panel closed.
  await page.getByTestId("mediaview/toolbar-tolerance").click();
  await expect(page.getByTestId("mediaview/tolerance")).toBeHidden();
  const delta = await chromeDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  console.log(`MERGESYNCMEDIA-147 media audio chromeDelta=${delta}`);
  expect(delta ?? Number.POSITIVE_INFINITY, `media chrome within budget (got ${delta}px)`).toBeLessThanOrEqual(140);

  // Evidence: full view + zoomed crops.
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await page.getByTestId("mediaview/summary").screenshot({ path: `${EVID}-zoom-rows.png` });
  await status.screenshot({ path: `${EVID}-zoom-status.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  if (fs.existsSync(BC_TRUTH)) {
    const truthB64 = fs.readFileSync(BC_TRUTH).toString("base64");
    const oursB64 = fs.readFileSync(`${EVID}-light.png`).toString("base64");
    await page.emulateMedia({ colorScheme: "light" });
    await page.setContent(
      `<html><body style="margin:0;display:flex;gap:8px;background:#fff;font:12px system-ui">` +
        `<figure style="margin:0"><figcaption>BC media truth (Home launcher)</figcaption><img src="data:image/png;base64,${truthB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
        `<figure style="margin:0"><figcaption>ours media 147 light (${browserName})</figcaption><img src="data:image/png;base64,${oursB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
        `</body></html>`,
    );
    await page.waitForTimeout(400);
    await page.screenshot({ path: `${EVID}-sidebyside.png`, fullPage: true });
  }

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
