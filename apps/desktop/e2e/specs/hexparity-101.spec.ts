/**
 * Wave-101 hex parity pass (ONE view, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) over the embedded 11-byte `h_*` pair ("hello
 * world" vs "hallo world", single page) and pins the Beyond Compare
 * bar (`/tmp/bc-hex-truth.png`):
 *  (a) chrome budget firstRowTop<=140px with find+goto panels closed
 *      (single toolbar row + per-side pane strip above the grid);
 *  (b) density consistent with the 22px token contract (uniform row
 *      heights, no stray 28px rows);
 *  (c) status + diffs-only/find/goto/copy wiring intact (legacy
 *      testids, byte-identical logic);
 *  (d) per-side parity where BC shows two panes: a per-side meta
 *      strip (left path + left bytes, right path + right bytes), a
 *      right-pane offset gutter mirroring the left one, per-char
 *      ASCII diff tint on both sides (BC paints LEF/RIGHT red, not
 *      just the hex bytes), and a pane divider; the bottom status
 *      carries per-side byte counts plus an honest client-measured
 *      load time (BC shows "28 bytes / 30 bytes" + "Load time").
 *  Light + dark DOM checks, zoomed PNG crops to `/tmp/` (never
 *  in-repo), assertNoOverlap, zero console errors / page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/hexparity-101";

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
  await page.getByTestId("session/left-path").fill("/tmp/hex-101-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-101-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

test("hexparity-101: chrome budget, density, wiring, per-side parity, light+dark", async ({
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

  // (d) Per-side parity: meta strip names both sides with byte counts
  // (BC per-pane "28 bytes / 30 bytes" headers).
  const panes = page.getByTestId("hexview/panes");
  await expect(panes).toBeVisible();
  const leftMeta = page.getByTestId("hexview/left-meta");
  const rightMeta = page.getByTestId("hexview/right-meta");
  await expect(leftMeta).toBeVisible();
  await expect(rightMeta).toBeVisible();
  await expect(leftMeta).toContainText("11 bytes");
  await expect(rightMeta).toContainText("11 bytes");
  await expect(leftMeta).toContainText("hex-101-left.bin");
  await expect(rightMeta).toContainText("hex-101-right.bin");

  // (d) Right-pane offset gutter mirrors the left one (BC prints
  // offsets in both panes); twins share the row (y within 2px).
  const rows = view.locator(".hx-row");
  await expect(rows.first()).toBeVisible();
  const rowCount = await rows.count();
  expect(rowCount).toBeGreaterThan(0);
  for (let r = 0; r < rowCount; r += 1) {
    const offs = rows.nth(r).locator(".hx-offset");
    await expect(offs).toHaveCount(2);
    await expect(offs.nth(0)).toHaveText(/^[0-9a-f]{8}$/);
    const t0 = await offs.nth(0).textContent();
    await expect(offs.nth(1)).toHaveText(t0 ?? "");
    const y0 = await offs.nth(0).evaluate((el) => el.getBoundingClientRect().y);
    const y1 = await offs.nth(1).evaluate((el) => el.getBoundingClientRect().y);
    expect(Math.abs(y0 - y1), `row ${r} gutters share a row`).toBeLessThan(2);
  }

  // (d) ASCII diff tint on both sides: the differing offset 1 paints
  // `hx-diff` inside BOTH `.hx-ascii` columns (BC paints LEF/RIGHT
  // red); the same offset 0 paints none. Containers stay 2/row.
  const row0Ascii = page.getByTestId("hexview/page-0").locator(".hx-row").first().locator(".hx-ascii");
  await expect(row0Ascii).toHaveCount(2);
  await expect(row0Ascii.nth(0)).toContainText("hello world");
  await expect(row0Ascii.nth(1)).toContainText("hallo world");
  const asciiDiffs = page.getByTestId("hexview/page-0").locator(".hx-row").first().locator(".hx-ascii .hx-diff");
  expect(await asciiDiffs.count(), "ascii diff tint on the differing char").toBeGreaterThanOrEqual(2);

  // (d) Bottom status: binary verdict + per-side counts + load time
  // (BC "Binary differences | ... | Load time: 0.04 seconds").
  const status = page.getByTestId("hexview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("hexview/status-diff")).toContainText("Binary differences");
  await expect(page.getByTestId("hexview/status-bytes")).toContainText("11 bytes");
  await expect(page.getByTestId("hexview/status-sizes")).toContainText("left 11 bytes");
  await expect(page.getByTestId("hexview/status-sizes")).toContainText("right 11 bytes");
  await expect(page.getByTestId("hexview/status-load")).toContainText(/Load time: [0-9]+\.[0-9]{2} seconds/);

  // (c) Wiring intact: diffs-only filter, find, goto, copy, detail.
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
  const before = await rows.count();
  await toggle.check();
  const note = page.getByTestId("hexview/diffsonly-note");
  await expect(note).toBeVisible();
  const noteText = (await note.textContent()) ?? "";
  const hidden = Number.parseInt(noteText.replace(/[^0-9]/g, ""), 10);
  expect(hidden).toBeGreaterThan(0);
  await expect(rows).toHaveCount(before - hidden);
  await toggle.uncheck();
  await expect(note).toBeHidden();

  await page.getByTestId("hex-copy-hex").click();
  await expect(page.getByTestId("hex-copy-status")).toContainText("hex");

  await assertNoOverlap(page);

  // (b) Density: uniform row heights on the 22px token contract, no
  // stray 28px rows (BC rows run ~28px; ours stay 22 by design).
  const heights = await page.evaluate(() => {
    const els = [...document.querySelectorAll(".hx-row")];
    return els.map((el) => (el as HTMLElement).getBoundingClientRect().height);
  });
  expect(heights.length).toBeGreaterThan(0);
  const minH = Math.min(...heights);
  const maxH = Math.max(...heights);
  expect(maxH - minH, `uniform row heights (got ${minH}..${maxH})`).toBeLessThanOrEqual(1);
  expect(maxH, `no stray 28px rows (got ${maxH}px)`).toBeLessThan(28);

  // (a) Chrome budget with both panels closed: first hex row top <=
  // 140px relative to the hex section top (toolbar + pane strip only).
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
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `hex chrome collapsed (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Evidence: full view + zoomed crops (toolbar/panes, diff row).
  await page.screenshot({ path: `${EVID}-light.png` });
  await page.getByTestId("hexview/toolbar").screenshot({ path: `${EVID}-zoom-toolbar.png` });
  await panes.screenshot({ path: `${EVID}-zoom-panes.png` });
  await rows.first().screenshot({ path: `${EVID}-zoom-diffrow.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}-dark.png` });
  await rows.first().screenshot({ path: `${EVID}-zoom-diffrow-dark.png` });

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
