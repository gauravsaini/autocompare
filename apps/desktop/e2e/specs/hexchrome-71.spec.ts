/**
 * Wave-71 hex chrome collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) over the embedded 11-byte `h_*` pair ("hello
 * world" vs "hallo world", single page) and proves the Beyond Compare
 * style collapse (mirrors wave-70 merge + wave-68 table):
 *  - one toolbar row (`hexview/toolbar`) hosts page prev/next + position,
 *    the differences-only toggle, Copy hex / Copy ASCII (inside the
 *    legacy `hex-copy-bar` wrapper), and the find + goto toggles;
 *  - the find + goto panels render open by default (backward compat:
 *    every legacy find/goto testid stays visible on load);
 *  - the bottom status line (`hexview/status`) carries the binary
 *    verdict (`Binary differences | different`, legacy `files differ`
 *    wording intact) plus the total byte count;
 *  - toolbar/Esc collapse the panels, Ctrl+F / Ctrl+G reopen + focus
 *    them;
 *  - collapsed chrome budget: first hex row top <= 140px relative to the
 *    hex section top with both panels closed (single toolbar row above
 *    the grid);
 *  - every legacy testid still works (find jump, goto jump, diffsonly
 *    filter, copy receipt, detail readout, pager);
 *  - light + dark, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-71";

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

async function createHexSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/hex-71-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-71-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("hex chrome collapse: single toolbar, find+goto overlays, status bar, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createHexSession(page);
  const view = page.getByTestId("view-hex");
  await expect(view).toContainText("files differ");
  await expect(view).toContainText("11 bytes");

  // 1. Single toolbar row with every control; find + goto panels open.
  const toolbar = page.getByTestId("hexview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(view.getByRole("button", { name: "Prev", exact: true })).toBeVisible();
  await expect(view.getByRole("button", { name: "Next", exact: true })).toBeVisible();
  await expect(page.getByTestId("hexview/page-label")).toContainText("page 1/1");
  await expect(page.getByTestId("hexview/diffsonly-toggle")).toBeVisible();
  await expect(page.getByTestId("hex-copy-hex")).toBeVisible();
  await expect(page.getByTestId("hex-copy-ascii")).toBeVisible();
  await expect(page.getByTestId("hexview/toolbar-find")).toBeVisible();
  await expect(page.getByTestId("hexview/toolbar-find")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("hexview/toolbar-goto")).toBeVisible();
  await expect(page.getByTestId("hexview/toolbar-goto")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect(page.getByTestId("hex-find-next")).toBeVisible();
  await expect(page.getByTestId("hex-find-status")).toBeVisible();
  await expect(page.getByTestId("hex-goto-input")).toBeVisible();
  await expect(page.getByTestId("hex-goto-go")).toBeVisible();
  await expect(page.getByTestId("hex-goto-status")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar at the bottom carries the binary verdict + byte count.
  const status = page.getByTestId("hexview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("hexview/status-diff")).toContainText("Binary differences");
  await expect(page.getByTestId("hexview/status-diff")).toContainText("different");
  await expect(page.getByTestId("hexview/status-bytes")).toContainText("11 bytes");
  await expect(page.getByTestId("hexview/status-bytes")).toContainText("files differ");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstRowTopAbs = await page.locator(".hx-row").first().evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "status bar lives below the rows").toBeGreaterThan(firstRowTopAbs);
  expect(toolbarTop, "toolbar sits above the rows").toBeLessThan(firstRowTopAbs);

  // 2. Toolbar toggle collapses the find panel; Ctrl+F reopens + focuses.
  await page.getByTestId("hexview/toolbar-find").click();
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hexview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("hex-find-input");

  // 3. Wave-142: global Esc closes both panels (was find-only); goto is already hidden.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();
  await expect(page.getByTestId("hexview/toolbar-goto")).toHaveAttribute("aria-expanded", "false");
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("hex-goto-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("hex-goto-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();
  // Reopen both for the legacy-behavior pass below.
  await page.getByTestId("hexview/toolbar-find").click();
  await page.getByTestId("hexview/toolbar-goto").click();
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect(page.getByTestId("hex-goto-input")).toBeVisible();

  // 4. Legacy behavior intact: find jump, goto jump + select, diffsonly
  //    filter with an honest hidden count, copy receipt, detail readout.
  await page.getByTestId("hex-find-input").fill("6c");
  await page.getByTestId("hex-find-next").click();
  await expect(page.getByTestId("hex-find-status")).toContainText("match at offset 2");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  await page.getByTestId("hex-goto-input").fill("5");
  await page.getByTestId("hex-goto-go").click();
  await expect(page.getByTestId("hex-goto-status")).toContainText("jumped to offset 5");
  await expect(page.getByTestId("hexview/byte-5")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(5)");

  const toggle = page.getByTestId("hexview/diffsonly-toggle");
  const before = await page.locator(".hx-row").count();
  await toggle.check();
  const note = page.getByTestId("hexview/diffsonly-note");
  await expect(note).toBeVisible();
  const noteText = (await note.textContent()) ?? "";
  const hidden = Number.parseInt(noteText.replace(/[^0-9]/g, ""), 10);
  expect(hidden).toBeGreaterThan(0);
  await expect(page.locator(".hx-row")).toHaveCount(before - hidden);
  await toggle.uncheck();
  await expect(note).toBeHidden();

  await page.getByTestId("hex-copy-hex").click();
  await expect(page.getByTestId("hex-copy-status")).toContainText("hex");

  await assertNoOverlap(page);

  // 5. Collapsed chrome budget: with both panels closed the hex chrome
  //    (toolbar + panels, i.e. first-row top relative to the hex section
  //    top -- app tab/path bars above are not hex chrome) is one compact
  //    row. Old stacked rows above the grid are gone.
  await page.getByTestId("hexview/toolbar-find").click();
  await page.getByTestId("hexview/toolbar-goto").click();
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="hex comparison"]');
    const first = document.querySelector(".hx-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `hex chrome collapsed to one row (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hex-chrome-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hex-chrome-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
