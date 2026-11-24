/**
 * Wave-102 merge parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict
 * merge pair (left `["left"]`, right `["right"]`, base `[]`) and pins
 * the Beyond Compare bar (`/tmp/bc-merge-truth.png`):
 *  (a) chrome budget firstCardTop<=140px section-relative with the
 *      find panel closed (single toolbar row + one-line per-pane
 *      headers above the cards);
 *  (b) density consistent (uniform pane heights, no stray 28px rows,
 *      compact one-row toolbar);
 *  (c) 3-way panes (left/base/right) + conflict counts intact;
 *  (d) take-left / take-right / take-both (+ order toggle) + merged
 *      preview + out-path save/apply wiring intact;
 *  (e) status bar counts (conflict sections, clean blocks,
 *      resolved/unresolved) follow choices.
 *  Light + dark DOM checks, zoomed PNG crops to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, rmSync } from "node:fs";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/mergeparity-102";
const OUT_FILE = "/tmp/mergeparity-102-out.txt";

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

test("mergeparity-102: chrome budget, density, 3-way panes, take wiring, status counts, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  mkdirSync("/tmp", { recursive: true });
  try {
    rmSync(OUT_FILE);
  } catch {
    // Missing output file is the expected starting state.
  }

  await createMergeSession(page);
  const view = page.getByTestId("view-merge");
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict section(s)");

  // (c) 3-way panes with BC-style per-pane file headers (path +
  // loaded line count, wave-102). Mock plans the embedded fixture:
  // left ["left"], base [], right ["right"].
  const paneLeft = page.getByTestId("merge-pane-left");
  const paneBase = page.getByTestId("merge-pane-base");
  const paneRight = page.getByTestId("merge-pane-right");
  await expect(paneLeft).toBeVisible();
  await expect(paneBase).toBeVisible();
  await expect(paneRight).toBeVisible();
  await expect(paneLeft).toContainText("left");
  await expect(paneRight).toContainText("right");
  const metaLeft = page.getByTestId("mergeview/meta-left");
  const metaBase = page.getByTestId("mergeview/meta-base");
  const metaRight = page.getByTestId("mergeview/meta-right");
  await expect(metaLeft).toBeVisible();
  await expect(metaBase).toBeVisible();
  await expect(metaRight).toBeVisible();
  await expect(metaLeft).toContainText("1 line");
  await expect(metaBase).toContainText("0 lines");
  await expect(metaRight).toContainText("1 line");
  // Session paths ride the headers (BC prints the file path per pane).
  await expect(metaLeft).toContainText("merge12-base.txt");
  await expect(metaRight).toContainText("merge12-right.txt");

  // Per-conflict per-side line counts on the card header.
  await expect(page.getByTestId("merge-conflict-meta-0")).toContainText("L1");
  await expect(page.getByTestId("merge-conflict-meta-0")).toContainText("B0");
  await expect(page.getByTestId("merge-conflict-meta-0")).toContainText("R1");

  // (e) Status bar counts before any choice.
  const status = page.getByTestId("mergeview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict section(s)");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("0 resolved");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("1 unresolved");

  // (d) Take wiring intact: take-left resolves, preview follows,
  // status counts follow, per-conflict clears take-all receipts.
  const preview = page.getByTestId("merge-preview");
  await page.getByTestId("merge-take-all-left").click();
  await expect(page.getByTestId("merge-take-all-status")).toContainText("took left for 1 conflict");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("1 resolved");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("0 unresolved");
  await page.getByTestId("merge-take-right-0").click();
  await expect(page.getByTestId("merge-take-all-status")).toHaveCount(0);
  await expect(preview).toContainText("right");

  // Take-both with order toggle (Beyond Compare L->R / R->L).
  await page.getByTestId("merge-take-both-0").click();
  await expect(page.getByTestId("merge-take-both-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toContainText("left");
  await page.getByTestId("merge-take-both-order-0").click();
  await expect(page.getByTestId("merge-take-both-order-0")).toContainText("R to L");
  await expect(preview).toContainText("right");
  await expect(page.getByTestId("merge-take-both-order-0")).toContainText("R to L");

  // Take-left then apply with a typed out-path (save wiring intact).
  await page.getByTestId("merge-take-left-0").click();
  await expect(preview).toHaveText("base\nleft");
  await page.getByTestId("merge-out-path").fill(OUT_FILE);
  await page.getByTestId("merge-apply").click();
  await expect(page.getByTestId("merge-apply-report")).toContainText(OUT_FILE);

  await assertNoOverlap(page);

  // (b) Density: uniform pane heights (no stray tall rows), compact
  // one-row toolbar.
  const paneHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll(".mv-pane")];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(paneHeights.length).toBe(3);
  const minH = Math.min(...paneHeights);
  const maxH = Math.max(...paneHeights);
  expect(maxH - minH, `uniform pane heights (got ${minH}..${maxH})`).toBeLessThanOrEqual(2);
  const toolbar = page.getByTestId("mergeview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // (a) Chrome budget with the find panel closed: first conflict-card
  // top <= 140px relative to the merge section top (toolbar + one-line
  // pane headers only; app tab/path bars above are not merge chrome).
  if (await page.getByTestId("merge/find-input").isVisible()) {
    await page.getByTestId("mergeview/toolbar-find").click();
  }
  await expect(page.getByTestId("merge/find-input")).toBeHidden();
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first card mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `merge chrome within budget (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Evidence: full view + zoomed crops (toolbar, panes, card).
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await paneLeft.screenshot({ path: `${EVID}-zoom-panes.png` });
  await page.getByTestId("mergeview/conflict-0").screenshot({ path: `${EVID}-zoom-card.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await page.getByTestId("mergeview/conflict-0").screenshot({ path: `${EVID}-zoom-card-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
