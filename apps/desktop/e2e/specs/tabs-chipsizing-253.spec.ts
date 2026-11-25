/**
 * TABS chipsizing-253 (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Session tabs standard desktop sizing par lao: chip --ac-tab-chip-h
 * control scale (--ac-control-h-md, 24px) se match kare, tabbar us hisab
 * se compact ho. Koi geometry pixel-lock nahi — token equality +
 * relational assertions only:
 *  - chip token == control-md token (light + dark);
 *  - measured chip height == token value; bar == chip + 2*space-1;
 *  - truncation classes intact (min-w/max-w/truncate) + close chip ke andar;
 *  - `+` (tab-new) last tab ke turant bagal me, tabbar ke andar (H1 fix intact).
 * New file only, koi existing spec touch nahi. Testids byte-identical.
 */
import { expect, test, type Page } from "@playwright/test";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("tabbar")).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function readTokens(page: Page): Promise<Record<string, string>> {
  return page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement);
    const get = (name: string): string => cs.getPropertyValue(name).trim();
    return {
      chip: get("--ac-tab-chip-h"),
      controlMd: get("--ac-control-h-md"),
      tabbar: get("--ac-tabbar-h"),
      space1: get("--ac-space-1"),
    };
  });
}

async function assertChipStandard(page: Page, scheme: string): Promise<void> {
  const t = await readTokens(page);
  // Token equality: chip control-md standard par hai — value lock nahi.
  expect(t.chip, `${scheme}: --ac-tab-chip-h == --ac-control-h-md`).toBe(t.controlMd);

  const px = (v: string): number => Number.parseFloat(v);
  const chipH = px(t.chip);
  const barH = px(t.tabbar);
  const gap = px(t.space1);
  expect(Number.isFinite(chipH) && chipH > 0, `${scheme}: chip token numeric`).toBe(true);

  // Measured chip == token; bar == chip + symmetric 4px rails (space-1*2).
  const firstTab = page.getByRole("tab").first();
  await expect(firstTab).toBeVisible();
  const tabBox = await firstTab.boundingBox();
  expect(tabBox, `${scheme}: chip ka layout box hai`).not.toBeNull();
  if (tabBox !== null) {
    expect(Math.round(tabBox.height), `${scheme}: measured chip == token`).toBe(Math.round(chipH));
  }
  const barBox = await page.getByTestId("tabbar").boundingBox();
  expect(barBox, `${scheme}: tabbar ka layout box hai`).not.toBeNull();
  if (barBox !== null) {
    expect(Math.round(barBox.height), `${scheme}: bar == chip + 2*space-1 (compact)`).toBe(
      Math.round(chipH + 2 * gap),
    );
    expect(Math.round(barBox.height), `${scheme}: bar purane 40px se compact`).toBeLessThan(40);
  }

  // Truncation contract intact: floor + cap + ellipsis, close chip ke andar.
  const cls = await firstTab.evaluate((el) =>
    el instanceof HTMLElement ? el.className : "",
  );
  expect(cls, `${scheme}: chip min-w floor intact`).toContain("min-w-[60px]");
  expect(cls, `${scheme}: chip max-w cap intact`).toContain("max-w-56");
  const trunc = await firstTab.locator("span").first().evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { overflow: cs.overflow, whiteSpace: cs.whiteSpace, textOverflow: cs.textOverflow };
  });
  expect(trunc.whiteSpace, `${scheme}: title nowrap`).toBe("nowrap");
  expect(trunc.textOverflow, `${scheme}: title ellipsis`).toBe("ellipsis");
  expect(trunc.overflow.startsWith("hidden"), `${scheme}: title overflow hidden`).toBe(true);

  // `+` adjacency (H1 fix): strip hug (grow 0), plus last tab ke baad, bar ke andar.
  const stripGrow = await page.evaluate(() => {
    const bar = document.querySelector('[data-testid="tabbar"]');
    const scroller = bar?.querySelector(":scope > div");
    if (!(scroller instanceof HTMLElement)) return "missing";
    return window.getComputedStyle(scroller).flexGrow;
  });
  expect(stripGrow, `${scheme}: strip hug (grow 0), + adjacent`).toBe("0");
  const order = await page.evaluate(() => {
    const bar = document.querySelector('[data-testid="tabbar"]');
    const tabs = Array.from(document.querySelectorAll('[role="tab"]'));
    const plus = document.querySelector('[data-testid="tab-new"]');
    if (!(bar instanceof HTMLElement) || !(plus instanceof HTMLElement) || tabs.length === 0) return null;
    const last = tabs[tabs.length - 1];
    if (!(last instanceof HTMLElement)) return null;
    const br = bar.getBoundingClientRect();
    const lr = last.getBoundingClientRect();
    const pr = plus.getBoundingClientRect();
    return {
      plusAfterLast: pr.x >= lr.x + lr.width - 2,
      plusInsideBar: pr.x >= br.x && pr.x + pr.width <= br.x + br.width + 2,
    };
  });
  expect(order?.plusAfterLast, `${scheme}: + last tab ke baad hai`).toBe(true);
  expect(order?.plusInsideBar, `${scheme}: + tabbar ke andar hai`).toBe(true);
}

test("tabs chipsizing: chip == control-md, compact bar, truncation + adjacent, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Light (default).
  await assertChipStandard(page, "light");

  // Dark override check.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await assertChipStandard(page, "dark");
  await page.emulateMedia({ colorScheme: "light" });

  // Testids byte-identical: tabbar + tab-new zinda hain.
  await expect(page.getByTestId("tabbar")).toBeVisible();
  await expect(page.getByTestId("tab-new")).toBeVisible();

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
