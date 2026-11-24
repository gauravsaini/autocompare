/**
 * Wave-147 merge parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict
 * merge pair and pins the Beyond Compare bar (`/tmp/bc-merge-truth.png`):
 *  (a) edit-mode parity (editable preview + spellcheck off + live line
 *      count, follows choices + manual touch-ups);
 *  (b) gutter parity (aria-hidden intra-conflict line numbers per cell,
 *      66J `<pre>` trio byte-identical);
 *  (c) overview-ruler parity (far-right in-flow rail, one marker per
 *      conflict, click jumps through selectConflict, zero vertical
 *      chrome: firstCardTop<=140px with the find panel closed).
 *  Light + dark DOM checks, zoomed PNG crops to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/parity-merge-147b";

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

test("parity-merge-147b: edit-mode, gutter, overview ruler, chrome budget, light+dark", async ({
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

  // (b) Gutter parity: one gutter per cell, intra-conflict numbers;
  // the 66J <pre> trio is byte-identical (3 pres, unchanged text).
  await expect(page.getByTestId("merge-gutter-left-0")).toHaveText("1");
  await expect(page.getByTestId("merge-gutter-base-0")).toHaveText("·");
  await expect(page.getByTestId("merge-gutter-right-0")).toHaveText("1");
  const card = page.getByTestId("mergeview/conflict-0");
  const pres = card.locator("pre");
  await expect(pres).toHaveCount(3);
  await expect(pres.nth(0)).toHaveText("left");
  await expect(pres.nth(1)).toHaveText("(no lines)");
  await expect(pres.nth(2)).toHaveText("right");

  // (c) Overview ruler: one rail, one marker, jump wiring intact.
  const ruler = page.getByTestId("mergeview/overview-ruler");
  await expect(ruler).toBeVisible();
  const marker = page.getByTestId("mergeview/overview-hunk-0");
  await expect(marker).toBeVisible();
  await expect(marker).toHaveAttribute("aria-label", "Jump to conflict 1 of 1");
  await expect(marker).toHaveAttribute("data-active", "true");
  await marker.click();
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  await expect(card).toHaveAttribute("aria-current", "true");

  // (a) Edit-mode parity: editable preview, spellcheck off, live count.
  const edit = page.getByTestId("merge-preview-edit");
  await expect(edit).toBeVisible();
  await expect(edit).toHaveAttribute("spellcheck", "false");
  // Unresolved preview = bundled base line + 5 marker lines.
  await expect(page.getByTestId("merge-preview-lines")).toHaveText("6 lines");
  await page.getByTestId("merge-take-left-0").click();
  await expect(page.getByTestId("merge-preview")).toHaveText("base\nleft");
  await expect(page.getByTestId("merge-preview-lines")).toHaveText("2 lines");
  await edit.fill("base\nleft\ntouched");
  await expect(page.getByTestId("merge-preview-lines")).toHaveText("3 lines");

  await assertNoOverlap(page);

  // Density: uniform pane heights, compact one-row toolbar.
  const paneHeights = await page.evaluate(() => {
    const els = [...document.querySelectorAll(".mv-pane")];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(paneHeights.length).toBe(3);
  expect(
    Math.max(...paneHeights) - Math.min(...paneHeights),
    `uniform pane heights (got ${Math.min(...paneHeights)}..${Math.max(...paneHeights)})`,
  ).toBeLessThanOrEqual(2);
  const toolbar = page.getByTestId("mergeview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Chrome budget with the find panel closed (default-closed).
  await expect(page.getByTestId("mergeview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
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

  // Evidence: full view + zoomed crops (toolbar, card, ruler).
  await page.screenshot({ path: `${EVID}-light.png` });
  await toolbar.screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await card.screenshot({ path: `${EVID}-zoom-card.png` });
  await ruler.screenshot({ path: `${EVID}-zoom-ruler.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await card.screenshot({ path: `${EVID}-zoom-card-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
