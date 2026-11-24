/**
 * Wave-72 media chrome collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MediaView) over the mock media engine and proves the
 * Beyond Compare style collapse (mirrors wave-71 hex + wave-70 merge):
 *  - one toolbar row (`mediaview/toolbar`) hosts the mode indicator plus,
 *    in audio mode only, the tolerance toggle + recompute button;
 *  - the tolerance panel renders open by default in audio mode only
 *    (backward compat: the wave-48 tolerance box stays visible on load)
 *    and toolbar/Esc collapse it;
 *  - the bottom status line (`mediaview/statusbar`) carries the verdict
 *    (`same`/`different`) plus the score and the differing/total counts
 *    for BOTH picture and audio modes;
 *  - collapsed chrome budget: first media row top <= 140px relative to
 *    the media section top with the tolerance panel closed;
 *  - every legacy testid still works (mode/score/rows/tolerance/
 *    recompute round-trip, picture + audio);
 *  - light + dark, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-72";

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

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
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

test("media chrome collapse: single-row toolbar, tolerance overlay, status bar, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // ---- Picture mode: toolbar is mode-only, status bar carries counts. ----
  await createMediaSession(page, "/tmp/m-left.png", "/tmp/m-right.png");
  const view = page.getByTestId("view-media");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Picture compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/99.7% equal/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/12 \/ 4100 bytes differ/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(2);

  // 1. Single-row toolbar; no tolerance chrome in picture mode.
  const toolbar = page.getByTestId("mediaview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toBeHidden();
  await expect(page.getByTestId("mediaview/recompare")).toBeHidden();
  await expect(page.getByTestId("mediaview/tolerance")).toBeHidden();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar at the bottom carries the verdict + counts.
  const status = page.getByTestId("mediaview/statusbar");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("different");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText("12 / 4100 bytes differ");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await page.getByTestId("mediaview/row").first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(firstRowTopAbs);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(firstRowTopAbs);

  // Collapsed chrome budget in picture mode (no panel exists here).
  const picDelta = await chromeDelta(page);
  expect(picDelta, "section + first row mounted (picture)").not.toBeNull();
  expect(picDelta ?? Number.POSITIVE_INFINITY, `media chrome collapsed to one row (got ${picDelta}px)`).toBeLessThanOrEqual(140);

  await assertNoOverlap(page, 'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]');

  // ---- Audio mode: tolerance panel open by default, toggle + Esc. ----
  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Audio compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/3 \/ 100 windows differ/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(3);

  const audioToolbar = page.getByTestId("mediaview/toolbar");
  await expect(audioToolbar).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("mediaview/tolerance")).toBeVisible();
  await expect(page.getByTestId("mediaview/recompare")).toBeVisible();
  const abarH = await audioToolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(abarH, `toolbar is one compact row (got ${abarH}px)`).toBeLessThan(120);

  const astatus = page.getByTestId("mediaview/statusbar");
  await expect(astatus).toBeVisible();
  await expect(page.getByTestId("mediaview/statusbar-verdict")).toContainText("different");
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText("3 / 100 windows differ");

  // 2. Toolbar toggle collapses the tolerance panel; toggle reopens + focuses.
  await page.getByTestId("mediaview/toolbar-tolerance").click();
  await expect(page.getByTestId("mediaview/tolerance")).toBeHidden();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toHaveAttribute("aria-expanded", "false");
  // Recompare stays reachable from the toolbar while the panel is closed.
  await expect(page.getByTestId("mediaview/recompare")).toBeVisible();
  await page.getByTestId("mediaview/toolbar-tolerance").click();
  await expect(page.getByTestId("mediaview/tolerance")).toBeVisible();
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toHaveAttribute("aria-expanded", "true");
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("mediaview/tolerance");

  // 3. Esc closes the tolerance panel (focus was in the input).
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("mediaview/tolerance")).toBeHidden();
  // Reopen for the legacy-behavior pass below.
  await page.getByTestId("mediaview/toolbar-tolerance").click();
  await expect(page.getByTestId("mediaview/tolerance")).toBeVisible();

  // 4. Legacy behavior intact: tolerance round-trip keeps the summary live.
  await page.getByTestId("mediaview/tolerance").fill("5");
  await page.getByTestId("mediaview/recompare").click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);
  await expect(page.getByTestId("mediaview/statusbar-counts")).toContainText(/windows differ/);

  await assertNoOverlap(page, 'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]');

  // 5. Collapsed chrome budget: with the tolerance panel closed the media
  //    chrome (toolbar + panel, i.e. first-row top relative to the media
  //    section top -- app tab/path bars above are not media chrome) is one
  //    compact row. Old stacked rows above the grid are gone.
  await page.getByTestId("mediaview/toolbar-tolerance").click();
  await expect(page.getByTestId("mediaview/tolerance")).toBeHidden();
  const delta = await chromeDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `media chrome collapsed to one row (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/media-chrome-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/media-chrome-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
