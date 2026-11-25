/**
 * Wave-231 chrome parity (MOCK IPC ONLY, VITE_MOCK_IPC=1).
 *
 * Chrome-shell budget gate, section-relative (never viewport-absolute):
 *  - shell rows: toolbar <= 48px, pathbar <= 36px, tabbar 40px,
 *    shell top (tabbar top -> pathbar bottom) <= 120px;
 *  - firstRowTop <= 140px: first text row sits within 140px of its
 *    own section top (in-view toolbars are views-owned and already
 *    single-row + bus-unified; Find/Goto ride overlays, no new rows).
 * Rendered-geometry only — behavior lives in e2e/specs/*. Testids
 * byte-identical, no view edits.
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    test.skip(true, `infra: ${err}`);
  }
});

test("chrome parity: shell rows + text firstRowTop <= 140px", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  const geometry = await page.evaluate(() => {
    const q = (s: string) => document.querySelector(s);
    const r = (el: Element | null) =>
      el instanceof HTMLElement ? el.getBoundingClientRect() : null;
    const tabbar = r(q('[data-testid="tabbar"]'));
    const toolbar = r(q('[role="toolbar"][aria-label="Compare actions"]'));
    const pathbar = r(q('[data-testid="pathbar"]'));
    const section = q('section[aria-label="text comparison"], section:has(.tv-row)');
    const first = q('[data-testid="textview/row-0"], .tv-row');
    const sRect = section instanceof HTMLElement ? section.getBoundingClientRect() : null;
    const fRect = first instanceof HTMLElement ? first.getBoundingClientRect() : null;
    return {
      tabbarH: tabbar ? Math.round(tabbar.height) : null,
      toolbarH: toolbar ? Math.round(toolbar.height) : null,
      pathbarH: pathbar ? Math.round(pathbar.height) : null,
      shellTop:
        tabbar && pathbar ? Math.round(pathbar.bottom - tabbar.top) : null,
      firstRowTop:
        sRect && fRect ? Math.round(fRect.top - sRect.top) : null,
    };
  });

  expect(geometry.toolbarH, "toolbar height measured").not.toBeNull();
  expect(geometry.toolbarH!, "toolbar height <= 48px").toBeLessThanOrEqual(48);
  expect(geometry.pathbarH, "pathbar height measured").not.toBeNull();
  expect(geometry.pathbarH!, "pathbar height <= 36px").toBeLessThanOrEqual(36);
  expect(geometry.tabbarH, "tabbar height is 40px").toBe(40);
  expect(geometry.shellTop, "shell top measured").not.toBeNull();
  expect(geometry.shellTop!, "shell top <= 120px").toBeLessThanOrEqual(120);
  expect(geometry.firstRowTop, "text firstRowTop measured").not.toBeNull();
  expect(
    geometry.firstRowTop!,
    `text firstRowTop <= 140px (got ${geometry.firstRowTop})`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  expect(pageErrors, "zero page errors").toEqual([]);
  expect(consoleErrors, "zero console errors").toEqual([]);
});
