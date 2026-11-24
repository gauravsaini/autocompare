/**
 * Wave-96 folder density BC 17px (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine and proves the
 * Beyond Compare folder pitch (observe-93 measured ~17 logical px):
 *  - every rendered folder row measures exactly 17px tall with a 17px
 *    row pitch (row1.top - row0.top), light + dark;
 *  - wrapped/equal behavior intact: collapse toggle hides descendants,
 *    diffs/same filters compose client-side, totals unchanged;
 *  - chrome budget intact: with the name panel closed the first row
 *    sits within 140px of the folder section top;
 *  - zoomed folder crop lands in `/tmp/folderdensity-96-zoom.png` and a
 *    side-by-side vs the BC folder truth lands in
 *    `/tmp/folderdensity-96-sidebyside.png` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const ZOOM_PNG = "/tmp/folderdensity-96-zoom.png";
const LIGHT_PNG = "/tmp/folderdensity-96-light.png";
const SIDEBYSIDE_PNG = "/tmp/folderdensity-96-sidebyside.png";
const BC_TRUTH = "/tmp/bc-folder-truth.png";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createFolderSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();
}

test("folder density-96: BC 17px rows, behavior intact, chrome budget, visuals", async ({
  page,
  browserName,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createFolderSession(page);
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(5);

  // 1. Density: every sampled row is exactly 17px tall with a 17px pitch.
  const density = await page.evaluate(() => {
    const nodes = Array.from(document.querySelectorAll('[data-testid^="folder-row-"]')).slice(0, 12);
    return nodes.map((n) => {
      const el = n as HTMLElement;
      const r = el.getBoundingClientRect();
      return { h: el.offsetHeight, rectH: r.height, top: r.top };
    });
  });
  expect(density.length).toBeGreaterThan(5);
  console.log(`DENSITY-96 heights=${density.map((d) => d.h).join(",")} pitch=${(density[1]!.top - density[0]!.top).toFixed(2)}`);
  for (const [idx, d] of density.entries()) {
    expect(d.h, `row ${idx} offsetHeight = 17px (got ${d.h})`).toBe(17);
    expect(d.rectH, `row ${idx} rect height = 17px (got ${d.rectH})`).toBeCloseTo(17, 0);
  }
  for (let idx = 1; idx < density.length; idx += 1) {
    const pitch = density[idx]!.top - density[idx - 1]!.top;
    expect(pitch, `row pitch ${idx - 1}->${idx} = 17px (got ${pitch})`).toBeCloseTo(17, 0);
  }

  // 2. Behavior intact: collapse + diffs/same filters, totals unchanged.
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedTotals = await totalsText();
  expect(loadedTotals).toMatch(/\d+\s*\/\s*\d+\s*entries/);
  const descendant = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  await expect(descendant).toHaveCount(1);
  const beforeCollapse = await rows.count();
  await page.getByTestId("folder-collapse-0").click();
  await expect(descendant).toHaveCount(0);
  expect(await rows.count(), "collapsing d00 hides its descendants").toBeLessThan(beforeCollapse);
  await page.getByTestId("folder-collapse-0").click();
  await expect(descendant).toHaveCount(1);
  const more = page.getByTestId("folderview/load-more");
  if ((await more.count()) > 0) {
    await more.first().click();
    await expect.poll(totalsText, { timeout: 10000 }).not.toBe(loadedTotals);
  }
  const fullTotals = await totalsText();
  await page.getByTestId("folder-filter-diffs").click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const diffStatuses: string[] = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
  for (const s of diffStatuses) {
    expect(s, "diffs filter hides every same row").not.toBe("same");
  }
  expect(await totalsText(), "filtering never refetches").toBe(fullTotals);
  await page.getByTestId("folder-filter-all").click();

  // 3. Chrome budget: panel closed, first row within 140px of section top.
  const nameFilter = page.getByTestId("folder-name-filter");
  if ((await nameFilter.count()) > 0 && (await nameFilter.isVisible())) {
    await page.getByTestId("folderview/toolbar-namefilter").click();
    await expect(nameFilter).toBeHidden();
  }
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  console.log(`DENSITY-96 chromeDelta=${chrome?.delta}`);
  expect(chrome?.delta ?? Number.POSITIVE_INFINITY, `firstRowTop<=140 (got ${chrome?.delta})`).toBeLessThanOrEqual(
    140,
  );

  // 4. Visuals: zoomed crop of the first rows + full light PNG.
  const firstBox = await rows.first().boundingBox();
  expect(firstBox, "first row has a bounding box").not.toBeNull();
  await page.screenshot({
    path: ZOOM_PNG,
    clip: {
      x: Math.max(0, firstBox!.x - 8),
      y: Math.max(0, firstBox!.y - 4),
      width: Math.min(900, 900),
      height: 10 * 17 + 8,
    },
  });
  await page.screenshot({ path: LIGHT_PNG });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  // Density holds in dark too (single sample: pitch of first two rows).
  const darkPitch = await page.evaluate(() => {
    const nodes = Array.from(document.querySelectorAll('[data-testid^="folder-row-"]')).slice(0, 2);
    if (nodes.length < 2) return null;
    const a = (nodes[0] as HTMLElement).getBoundingClientRect();
    const b = (nodes[1] as HTMLElement).getBoundingClientRect();
    return { ha: (nodes[0] as HTMLElement).offsetHeight, pitch: b.top - a.top };
  });
  expect(darkPitch, "dark rows mounted").not.toBeNull();
  expect(darkPitch!.ha, `dark row height = 17px (got ${darkPitch!.ha})`).toBe(17);
  expect(darkPitch!.pitch, `dark row pitch = 17px (got ${darkPitch!.pitch})`).toBeCloseTo(17, 0);

  // 5. Side-by-side vs BC truth (stand-in: our light PNG if truth missing).
  const truthPath = fs.existsSync(BC_TRUTH) ? BC_TRUTH : LIGHT_PNG;
  const truthNote = fs.existsSync(BC_TRUTH) ? "BC folder truth (/tmp/bc-folder-truth.png)" : "STAND-IN: our light PNG (BC truth missing)";
  const truthB64 = fs.readFileSync(truthPath).toString("base64");
  const oursB64 = fs.readFileSync(LIGHT_PNG).toString("base64");
  await page.emulateMedia({ colorScheme: "light" });
  await page.setContent(
    `<html><body style="margin:0;display:flex;gap:8px;background:#fff;font:12px system-ui">` +
      `<figure style="margin:0"><figcaption>${truthNote}</figcaption><img src="data:image/png;base64,${truthB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
      `<figure style="margin:0"><figcaption>ours 17px folder (${browserName})</figcaption><img src="data:image/png;base64,${oursB64}" style="max-width:620px;border:1px solid #888"/></figure>` +
      `</body></html>`,
  );
  await page.waitForTimeout(400);
  await page.screenshot({ path: SIDEBYSIDE_PNG, fullPage: true });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
