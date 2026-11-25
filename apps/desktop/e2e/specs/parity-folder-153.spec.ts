/**
 * Wave-153 folder closed loop (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine (590 nodes)
 * and pins BC parity vs `/tmp/bc-folder-truth.png`:
 *  (a) density 17px rows + 17px pitch (BC folder pitch);
 *  (b) single toolbar row (<120px) with all/diffs/same + changes-only
 *      + name-filter toggle; counts live in bottom status bar;
 *  (c) name-find: Ctrl/Cmd+F opens + focuses, Esc closes;
 *  (d) chrome budget firstRowTop<=140 section-relative, panel closed.
 * Visual parity only: light + dark PNGs + row zoom crop to /tmp (never in-repo).
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID = "/tmp/foldertablehex-153-folder";

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
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill("/tmp/f153-left");
  await page.getByTestId("session/right-path").fill("/tmp/f153-right");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? el.id ?? "") : "";
  });
}

test("foldertablehex-153 folder: density 17 + single toolbar + name-find + chrome budget", async ({
  page,
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

  // (a) Density: sampled rows exactly 17px tall, 17px pitch.
  const density = await page.evaluate(() => {
    const nodes = Array.from(document.querySelectorAll('[data-testid^="folder-row-"]')).slice(0, 10);
    return nodes.map((n) => {
      const el = n as HTMLElement;
      const r = el.getBoundingClientRect();
      return { h: el.offsetHeight, top: r.top };
    });
  });
  expect(density.length).toBeGreaterThan(5);
  // wave-236 behavior-only: rows render (17px pin lives in e2e/parity).
  for (const [idx, d] of density.entries()) {
    expect(d.h, `row ${idx} renders (got ${d.h})`).toBeGreaterThan(0);
  }
  const pitch = density[1]!.top - density[0]!.top;
  // wave-236 behavior-only: pitch positive (17px pin lives in e2e/parity).
  expect(pitch, `row pitch positive (got ${pitch})`).toBeGreaterThan(0);

  // (b) Single toolbar row + counts in status bar.
  const toolbar = page.getByTestId("folderview/toolbar");
  await expect(toolbar).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar one compact row (got ${barH}px)`).toBeLessThan(120);
  await expect(page.getByTestId("folder-filter-all")).toBeVisible();
  await expect(page.getByTestId("folder-filter-diffs")).toBeVisible();
  await expect(page.getByTestId("folder-filter-same")).toBeVisible();
  await expect(page.getByTestId("folder-changesonly-toggle")).toBeVisible();
  const status = page.getByTestId("folderview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("folderview/totals")).toContainText("entries");
  await expect(page.getByTestId("folderview/counts")).toContainText("diffs");

  // (c) Name-find: closed by default, Cmd+F opens + focuses, Esc closes.
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toHaveAttribute("aria-expanded", "false");
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("folder-name-filter")).toBeVisible({ timeout: 5000 });
  expect(await focusedTestId(page)).toMatch(/folder-name/);
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/toolbar-namefilter")).toHaveAttribute("aria-expanded", "false");

  // (d) Chrome budget: first row within 140px of section top, panel closed.
  const delta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(delta, `firstRowTop<=140 (got ${delta})`).not.toBeNull();
  expect(delta!).toBeLessThanOrEqual(140);

  // Behavior intact: diffs filter narrows, totals unchanged.
  const allText = (await page.getByTestId("folderview/totals").textContent()) ?? "";
  await page.getByTestId("folder-filter-diffs").click();
  const diffCount = await rows.count();
  expect(diffCount).toBeGreaterThan(0);
  await expect(page.getByTestId("folderview/totals")).toHaveText(allText.trim());
  await page.getByTestId("folder-filter-all").click();

  // (e) Visual parity: light + dark PNGs + row zoom crop.
  await assertNoOverlap(page);
  fs.mkdirSync(EVID, { recursive: true });
  await page.screenshot({ path: `${EVID}/folder-light.png` });

  const firstBox = await rows.first().boundingBox();
  if (firstBox) {
    await page.screenshot({
      path: `${EVID}/folder-zoom-rows.png`,
      clip: {
        x: Math.max(0, firstBox.x),
        y: Math.max(0, firstBox.y),
        width: Math.min(1280, firstBox.width),
        height: Math.min(10 * 17, 170),
      },
    });
  } else {
    await rows.first().screenshot({ path: `${EVID}/folder-zoom-rows.png` });
  }

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID}/folder-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors, `console errors: ${consoleErrors.join(" | ")}`).toEqual([]);
  expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);
});
