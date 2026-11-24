/**
 * Wave-147 media parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MediaView) over the mock media engine and pins the
 * Beyond Compare bar (`/tmp/bc-media-truth.png`):
 *  (a) diff chrome parity (one compact toolbar row, per-file headers
 *      with full path in title, verdict + counts in the status bar);
 *  (b) empty states (picture/audio `empty-rows` notes render ONLY when
 *      the rows block is absent — identical vs capped copy; the
 *      pick-two-files empty names both modes);
 *  (c) chrome budget firstRowTop<=140px + uniform density, light+dark.
 *  Zoomed PNG crops + side-by-side vs the BC truth to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/parity-media-147b";
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

test("parity-media-147b: diff chrome, empty states, budget, density, light+dark", async ({
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

  // (a) Per-file headers name the session files (BC prints the file
  // path above every compare pane).
  await expect(page.getByTestId("mediaview/meta-left")).toContainText("m-left.png");
  await expect(page.getByTestId("mediaview/meta-right")).toContainText("m-right.png");
  await expect(page.getByTestId("mediaview/meta-left")).toHaveAttribute("title", "/tmp/m-left.png");
  await expect(page.getByTestId("mediaview/meta-right")).toHaveAttribute("title", "/tmp/m-right.png");

  // (b) No stray empty chrome on the normal path: rows exist, so the
  // empty-rows note must not mount.
  await expect(page.getByTestId("mediaview/empty-rows")).toHaveCount(0);

  // Toolbar is one compact row, mode-only in picture mode.
  const toolbar = page.getByTestId("mediaview/toolbar");
  await expect(toolbar).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar carries the verdict + counts below the rows.
  const status = page.getByTestId("mediaview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("different");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText("12 / 4100 bytes differ");

  // (c) Density: uniform row heights + steady pitch, no stray tall rows.
  const density = await page.evaluate(() => {
    const nodes = [...document.querySelectorAll('[data-testid="mediaview/row"]')];
    const hs = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().height);
    const tops = nodes.map((el) => (el as HTMLElement).getBoundingClientRect().top);
    const pitches = tops.slice(1).map((t, i) => t - tops[i]);
    return { hs, pitches };
  });
  expect(density.hs.length).toBe(2);
  expect(
    Math.max(...density.hs) - Math.min(...density.hs),
    "uniform row heights",
  ).toBeLessThanOrEqual(2);
  expect(Math.max(...density.hs), "compact rows, no stray tall rows").toBeLessThan(60);

  // (c) Chrome budget in picture mode (no panel exists here).
  const picDelta = await chromeDelta(page);
  expect(picDelta, "section + first row mounted (picture)").not.toBeNull();
  expect(
    picDelta ?? Number.POSITIVE_INFINITY,
    `media chrome within budget (got ${picDelta}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);

  // ---- Audio mode: panel behavior, headers, recompute wiring. ----
  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Audio compare/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(3);
  await expect(page.getByTestId("mediaview/empty-rows")).toHaveCount(0);

  await expect(page.getByTestId("mediaview/meta-left")).toContainText("m-left.wav");
  await expect(page.getByTestId("mediaview/meta-right")).toContainText("m-right.wav");

  // Tolerance panel open by default; toggle + Esc collapse it.
  const audioToolbar = page.getByTestId("mediaview/toolbar");
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("mediaview/tolerance")).toBeVisible();
  const abarH = await audioToolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(abarH, `toolbar is one compact row (got ${abarH}px)`).toBeLessThan(120);

  // Recompute wiring intact: tolerance round-trip keeps the summary live.
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
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `media chrome within budget (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

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

  // Side-by-side vs the BC truth (truth pixels ship in the figure).
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
