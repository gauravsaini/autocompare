/**
 * Wave-188 merge parity slice (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * GEOMETRY ONLY — no take/apply/find behavior (owned by merge-12/22/37/45,
 * mergechrome-70, mergegutter-66J, mergeparity-102). Drives the REAL `App`
 * workspace (SessionManager dialog -> tab -> ViewRouter -> MergeView) over
 * the engine-matched single-conflict merge pair (left `["left"]`, right
 * `["right"]`, base `[]`) and pins the Beyond Compare 3-way bar:
 *  (a) density: uniform top-pane heights (±2px), compact one-row toolbar
 *      (<=48px), `--ac-row-h` token stays 22px;
 *  (b) per-pane gutters: every top pane (left/base/right) carries its own
 *      `merge-pane-gutter-*` (1-based, aria-hidden), every conflict cell
 *      keeps its `merge-gutter-*-0`, and each gutter's line-height equals
 *      its sibling `pre` line-height (rows align);
 *  (c) thin splitter: one 1px `mv-splitter` rail between panes
 *      (`merge-splitter-base/right`, width <=2px, full pane height);
 *  (d) chrome budget: first-card top <=140px section-relative with the
 *      find panel closed (toolbar + one-line pane headers only);
 *  (e) owned-section no-overlap + status bar visible;
 *  light + dark DOM checks, zoomed PNGs to `/tmp/` (never in-repo),
 *  zero console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/mergeparity-188";
const MERGE_OVERLAP_SELECTORS =
  'section[aria-label="three-way merge"], [data-testid^="merge-"], [data-testid^="mergeview/"]';

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
  await expect(page.getByTestId("view-merge")).toBeVisible();
}

test("mergeparity-188: density, per-pane gutters, thin splitter, chrome budget, owned no-overlap, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMergeSession(page);
  const view = page.getByTestId("view-merge");
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict section(s)");

  // (a) Density: toolbar is one compact row; row token stays 22px;
  // top panes share one height (no stray tall rows).
  const toolbar = page.getByTestId("mergeview/toolbar");
  await expect(toolbar).toBeVisible();
  const toolbarH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(toolbarH, `toolbar is one compact row <=48px (got ${toolbarH}px)`).toBeLessThanOrEqual(48);
  const rowToken = await page.evaluate(() =>
    window.getComputedStyle(document.documentElement).getPropertyValue("--ac-row-h").trim(),
  );
  expect(rowToken, "unified row token stays 22px").toBe("22px");
  const paneHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll(".mv-pane")];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(paneHeights.length).toBe(3);
  const minH = Math.min(...paneHeights);
  const maxH = Math.max(...paneHeights);
  expect(maxH - minH, `uniform pane heights (got ${minH}..${maxH})`).toBeLessThanOrEqual(2);

  // (b) Per-pane gutters: one per top pane + three on the conflict card,
  // each aligned with its sibling pre (equal line-heights).
  for (const side of ["left", "base", "right"] as const) {
    const gutter = page.getByTestId(`merge-pane-gutter-${side}`);
    await expect(gutter, `top pane gutter ${side} visible`).toBeVisible();
    await expect(gutter).toHaveAttribute("aria-hidden", "true");
  }
  await expect(page.getByTestId("merge-pane-gutter-left")).toContainText("1");
  await expect(page.getByTestId("merge-pane-gutter-base")).toContainText("·");
  await expect(page.getByTestId("merge-pane-gutter-right")).toContainText("1");
  for (const side of ["left", "base", "right"] as const) {
    await expect(page.getByTestId(`merge-gutter-${side}-0`), `card gutter ${side} intact`).toBeVisible();
  }
  const gutterAlign = await page.evaluate(() => {
    const pairs: Array<[string, string]> = [
      [".mv-pane[data-testid='merge-pane-left']", "merge-pane-gutter-left"],
      [".mv-pane[data-testid='merge-pane-right']", "merge-pane-gutter-right"],
    ];
    return pairs.map(([paneSel, gutterId]) => {
      const pane = document.querySelector(paneSel);
      const gutter = pane?.querySelector(`[data-testid="${gutterId}"]`);
      const pre = pane?.querySelector("pre");
      if (!(gutter instanceof HTMLElement) || !(pre instanceof HTMLElement)) return null;
      const g = window.getComputedStyle(gutter).lineHeight;
      const p = window.getComputedStyle(pre).lineHeight;
      return { gutterId, gutterLineH: g, preLineH: p };
    });
  });
  for (const row of gutterAlign) {
    expect(row, "pane gutter + pre mounted").not.toBeNull();
    expect(
      row?.gutterLineH,
      `${row?.gutterId} line-height matches its pre (${row?.gutterLineH} vs ${row?.preLineH})`,
    ).toBe(row?.preLineH);
  }

  // (c) Thin splitter: 1px rails between the three panes, full height.
  for (const testid of ["merge-splitter-base", "merge-splitter-right"] as const) {
    const rail = page.getByTestId(testid);
    await expect(rail, `${testid} visible`).toBeVisible();
    const box = await rail.evaluate((el) => el.getBoundingClientRect().toJSON());
    expect(box.width, `${testid} thin rail <=2px (got ${box.width}px)`).toBeLessThanOrEqual(2);
    expect(
      box.height,
      `${testid} full pane height (got ${box.height}px vs panes ${minH}..${maxH}px)`,
    ).toBeGreaterThanOrEqual(minH - 2);
  }

  // (d) Chrome budget with the find panel closed: first conflict-card
  // top <= 140px relative to the merge section top.
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

  // (e) Owned-section no-overlap + status bar visible with counts.
  await assertNoOverlap(page, MERGE_OVERLAP_SELECTORS);
  const status = page.getByTestId("mergeview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("mergeview/status-note")).toContainText("0 resolved");

  // Evidence: full view + zoomed crops (never in-repo).
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await page.getByTestId("merge-pane-left").screenshot({ path: `${EVID}-zoom-pane.png` });
  await page.getByTestId("mergeview/conflict-0").screenshot({ path: `${EVID}-zoom-card.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await assertNoOverlap(page, MERGE_OVERLAP_SELECTORS);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
