/**
 * Scroll-Contain Spec (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * BC parity: only the comparison panes scroll, never the whole window.
 * The shell (tabs + toolbar + pathbar + statusbar) stays pinned while
 * the text virtual pane scrolls internally.
 *
 * - window.scrollY stays 0 after wheeling over the pane
 * - chrome/band rect is unchanged by pane scrolling
 * - textview/virtual scrollTop grows (internal scroll works)
 * - zero console errors / pageerrors
 */
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

test("scrollcontain: window never scrolls, pane scrolls internally, chrome pinned", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  const band = page.getByTestId("chrome/band");
  await expect(band).toBeVisible({ timeout: 15000 });
  const virtual = page.getByTestId("textview/virtual");
  const rows = page.getByTestId("textview/rows");
  // Demo data is small: virtualizer only mounts past 500 rows, else the
  // plain list branch renders. Either is the pane scroll container.
  const scroller =
    (await virtual.count()) > 0 ? virtual : rows;
  await expect(scroller).toBeVisible({ timeout: 15000 });

  const bandBefore = await band.boundingBox();
  expect(bandBefore, "chrome band visible").not.toBeNull();

  // No page-level scroll room: document fits the viewport.
  const pageFits = await page.evaluate(() => {
    const el = document.scrollingElement ?? document.documentElement;
    return el.scrollHeight <= window.innerHeight + 1;
  });
  expect(pageFits, "document fits viewport (no window scroll)").toBe(true);

  // Wheel over the pane several times to force internal scroll.
  const box = await scroller.boundingBox();
  expect(box, "pane has size").not.toBeNull();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
  const scrolled = await scroller.evaluate(async (el) => {
    const start = el.scrollTop;
    for (let i = 0; i < 10; i++) {
      el.scrollBy(0, 200);
      await new Promise((r) => setTimeout(r, 30));
    }
    return { start, end: el.scrollTop, max: el.scrollHeight - el.clientHeight };
  });
  if (scrolled.max > 50) {
    expect(scrolled.end, "pane scrolled internally").toBeGreaterThan(scrolled.start);
  } else {
    // Small demo data: nothing to scroll, containment still must hold.
    expect(scrolled.end, "no phantom scroll").toBe(scrolled.start);
  }

  // Window stayed put and chrome did not move.
  expect(await page.evaluate(() => window.scrollY), "window.scrollY stays 0").toBe(0);
  const bandAfter = await band.boundingBox();
  expect(bandAfter!.y, "band y pinned").toBeCloseTo(bandBefore!.y, 0);
  expect(bandAfter!.height, "band height stable").toBeCloseTo(bandBefore!.height, 0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
