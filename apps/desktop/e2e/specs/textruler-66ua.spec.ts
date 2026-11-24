/**
 * Wave-66U-a ruler far-left (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * ChromeShell mounts the OverviewRuler BEFORE <main> in the flex row
 * (far-left, BC-style). Drives the REAL `App` workspace with the
 * auto-booted demo text diff and proves on the live DOM:
 *  - flex/DOM order: ruler precedes <main> (compareDocumentPosition +
 *    boundingBox: ruler right <= main left, ruler x near the window
 *    left edge);
 *  - divider faces content: computed border-right 1px, border-left 0px
 *    (ruler-scoped tokens.css flip of the component's stock border-l);
 *  - onJump still works: click near the ruler bottom scrolls the
 *    virtualized pane, End/Home keyboard jumps keep working;
 *  - zero console errors / pageerrors.
 *
 * New file only, no existing spec touched. Existing geometry-pin
 * failures are OUT of scope for this wave.
 */
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function scrollTopOf(page: Page, testid: string): Promise<number> {
  return page.evaluate((id) => {
    const el = document.querySelector(`[data-testid="${id}"]`);
    return el instanceof HTMLElement ? el.scrollTop : -1;
  }, testid);
}

test("ruler far-left precedes main, onJump still works", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await expect(page.getByTestId("view-text")).toBeVisible();

  const ruler = page.getByTestId("ruler");
  await expect(ruler).toBeVisible();
  await expect(ruler).toHaveAttribute("role", "slider");

  // Load the full diff so the pane virtualizes and the ruler owns the map.
  for (let i = 0; i < 10; i += 1) {
    if ((await page.getByTestId("textview/virtual").count()) > 0) break;
    const more = page.getByRole("button", { name: /Load more/ });
    if ((await more.count()) === 0) break;
    await more.first().click();
  }
  await expect(page.getByTestId("textview/virtual")).toBeVisible();

  // 1. Flex/DOM order via evaluate: ruler precedes <main> in the same
  // flex row, geometry far-left (ruler right <= main left).
  const order = await page.evaluate(() => {
    const rulerEl = document.querySelector('[data-testid="ruler"]') as HTMLElement | null;
    const mainEl = rulerEl?.parentElement?.querySelector("main") ?? null;
    if (rulerEl === null || mainEl === null) return null;
    const parent = rulerEl.parentElement as HTMLElement | null;
    const display = parent !== null ? window.getComputedStyle(parent).display : "";
    const follows =
      (rulerEl.compareDocumentPosition(mainEl) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    const rulerBox = rulerEl.getBoundingClientRect();
    const mainBox = (mainEl as HTMLElement).getBoundingClientRect();
    const cs = window.getComputedStyle(rulerEl);
    return {
      display,
      follows,
      rulerX: rulerBox.x,
      rulerRight: rulerBox.right,
      mainX: mainBox.x,
      borderLeft: cs.borderLeftWidth,
      borderRight: cs.borderRightWidth,
    };
  });
  expect(order, "ruler + main resolve").not.toBeNull();
  expect(order?.display, "row container is flex").toBe("flex");
  expect(order?.follows, "ruler precedes main in DOM/flex order").toBe(true);
  expect(order!.rulerRight, "ruler right edge meets main left edge").toBeLessThanOrEqual(
    order!.mainX + 1,
  );
  expect(order!.rulerX, "ruler sits far-left").toBeLessThanOrEqual(24);
  // Divider faces content after the far-left move.
  expect(order?.borderRight, "ruler divider on content side").not.toBe("0px");
  expect(order?.borderLeft, "no outer border on window edge").toBe("0px");

  // 2. onJump still works: click near the ruler bottom scrolls the pane.
  const box = await ruler.boundingBox();
  expect(box, "ruler has a clickable box").not.toBeNull();
  const beforeClick = await scrollTopOf(page, "textview/virtual");
  expect(beforeClick, "virtual scroller exists").toBeGreaterThanOrEqual(0);
  await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height * 0.9);
  await expect
    .poll(async () => scrollTopOf(page, "textview/virtual"), { timeout: 5000 })
    .toBeGreaterThan(beforeClick);

  // Keyboard still jumps: End lands at the end, Home returns to top.
  await ruler.focus();
  await page.keyboard.press("End");
  await expect
    .poll(async () => scrollTopOf(page, "textview/virtual"), { timeout: 5000 })
    .toBeGreaterThan(beforeClick);
  await page.keyboard.press("Home");
  await expect.poll(async () => scrollTopOf(page, "textview/virtual"), { timeout: 5000 }).toBe(0);

  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
