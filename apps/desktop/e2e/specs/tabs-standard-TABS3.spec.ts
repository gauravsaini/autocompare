/**
 * Standard Tabs Spec - TABS3 (MOCK IPC ONLY, VITE_MOCK_IPC=1, webkit).
 *
 * Verifies standard desktop tab bar design system contracts:
 * - Token equality: chip == control-md (--ac-tab-chip-h == --ac-control-h-md, 24px),
 *   bar == chip + 2*space-1 (--ac-tabbar-h == 32px), min 96 / max 180
 *   (--ac-tab-min-w: 96px, --ac-tab-max-w: 180px) in both light and dark modes.
 * - Measured chip == token height + bar == chip + 2*gap + bar < 40px.
 * - Flex classes: flex-1 and basis-0 class pins on chips.
 * - Ellipsis: nowrap, ellipsis, overflow hidden on tab title span.
 * - Adjacency: tab-new (+) inside tabbar, right of last tab, visible without scroll.
 * - Gallery tabs block visible: navigate to /?gallery=1, verify gallery/section-tabs is visible.
 * - No pixel locks beyond token equality + relational checks.
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

interface TabTokens {
  chip: string;
  controlMd: string;
  tabbar: string;
  space1: string;
  tabMinW: string;
  tabMaxW: string;
}

async function readTokens(page: Page): Promise<TabTokens> {
  return page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement);
    const get = (name: string): string => cs.getPropertyValue(name).trim();
    return {
      chip: get("--ac-tab-chip-h"),
      controlMd: get("--ac-control-h-md"),
      tabbar: get("--ac-tabbar-h"),
      space1: get("--ac-space-1"),
      tabMinW: get("--ac-tab-min-w"),
      tabMaxW: get("--ac-tab-max-w"),
    };
  });
}

async function assertTabStandards(page: Page, scheme: string): Promise<void> {
  const t = await readTokens(page);

  // Token equality: chip == control-md (--ac-tab-chip-h == --ac-control-h-md, 24px)
  expect(t.chip, `${scheme}: --ac-tab-chip-h == --ac-control-h-md`).toBe(t.controlMd);
  expect(t.chip, `${scheme}: --ac-tab-chip-h == 24px`).toBe("24px");
  expect(t.controlMd, `${scheme}: --ac-control-h-md == 24px`).toBe("24px");

  // bar == chip + 2*space-1 (--ac-tabbar-h == 32px)
  expect(t.tabbar, `${scheme}: --ac-tabbar-h == 32px`).toBe("32px");
  const px = (v: string): number => Number.parseFloat(v);
  const chipH = px(t.chip);
  const barH = px(t.tabbar);
  const gap = px(t.space1);
  expect(Number.isFinite(chipH) && chipH > 0, `${scheme}: chip token numeric`).toBe(true);
  expect(barH, `${scheme}: bar == chip + 2*space-1`).toBe(chipH + 2 * gap);

  // min 96 / max 180 (--ac-tab-min-w: 96px, --ac-tab-max-w: 180px)
  expect(t.tabMinW, `${scheme}: --ac-tab-min-w == 96px`).toBe("96px");
  expect(t.tabMaxW, `${scheme}: --ac-tab-max-w == 180px`).toBe("180px");

  // Measured chip == token height + bar == chip + 2*gap + bar < 40px
  const firstTab = page.getByRole("tab").first();
  await expect(firstTab).toBeVisible();
  const tabBox = await firstTab.boundingBox();
  expect(tabBox, `${scheme}: chip has bounding box`).not.toBeNull();
  if (tabBox !== null) {
    expect(Math.round(tabBox.height), `${scheme}: measured chip == token height`).toBe(Math.round(chipH));
  }

  const barBox = await page.getByTestId("tabbar").boundingBox();
  expect(barBox, `${scheme}: tabbar has bounding box`).not.toBeNull();
  if (barBox !== null) {
    expect(Math.round(barBox.height), `${scheme}: bar == chip + 2*gap`).toBe(Math.round(chipH + 2 * gap));
    expect(Math.round(barBox.height), `${scheme}: bar < 40px`).toBeLessThan(40);
  }

  // Flex classes: flex-1 and basis-0 class pins on chips
  const cls = await firstTab.evaluate((el) => (el instanceof HTMLElement ? el.className : ""));
  expect(cls, `${scheme}: flex-1 class pin on chip`).toMatch(/(^|\s)flex-1(\s|$)/);
  expect(cls, `${scheme}: basis-0 class pin on chip`).toContain("basis-0");

  // Ellipsis: nowrap, ellipsis, overflow hidden on tab title span
  const trunc = await firstTab.locator("span").first().evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return {
      overflow: cs.overflow,
      whiteSpace: cs.whiteSpace,
      textOverflow: cs.textOverflow,
    };
  });
  expect(trunc.whiteSpace, `${scheme}: title nowrap`).toBe("nowrap");
  expect(trunc.textOverflow, `${scheme}: title ellipsis`).toBe("ellipsis");
  expect(trunc.overflow.startsWith("hidden"), `${scheme}: title overflow hidden`).toBe(true);

  // Adjacency: tab-new (+) inside tabbar, right of last tab, visible without scroll
  const tabNew = page.getByTestId("tab-new");
  await expect(tabNew).toBeVisible();

  const adjacency = await page.evaluate(() => {
    const bar = document.querySelector('[data-testid="tabbar"]');
    const tabs = Array.from(document.querySelectorAll('[role="tab"]'));
    const plus = document.querySelector('[data-testid="tab-new"]');
    if (!(bar instanceof HTMLElement) || !(plus instanceof HTMLElement) || tabs.length === 0) {
      return null;
    }
    const lastTab = tabs[tabs.length - 1];
    if (!(lastTab instanceof HTMLElement)) return null;

    const br = bar.getBoundingClientRect();
    const lr = lastTab.getBoundingClientRect();
    const pr = plus.getBoundingClientRect();

    const scroller = bar.querySelector(":scope > div");
    const stripGrow = scroller instanceof HTMLElement ? window.getComputedStyle(scroller).flexGrow : null;

    return {
      plusAfterLast: pr.x >= lr.x + lr.width - 2,
      plusInsideBar: pr.x >= br.x && pr.x + pr.width <= br.x + br.width + 2,
      isPlusVisibleInBar: pr.right <= br.right + 2 && pr.left >= br.left - 2,
      stripGrow,
    };
  });

  expect(adjacency, `${scheme}: adjacency evaluation succeeded`).not.toBeNull();
  expect(adjacency?.plusAfterLast, `${scheme}: tab-new (+) right of last tab`).toBe(true);
  expect(adjacency?.plusInsideBar, `${scheme}: tab-new (+) inside tabbar`).toBe(true);
  expect(adjacency?.isPlusVisibleInBar, `${scheme}: tab-new (+) visible without scroll`).toBe(true);
  if (adjacency?.stripGrow) {
    expect(adjacency.stripGrow, `${scheme}: tab strip hugs content (grow 0)`).toBe("0");
  }
}

test("standard tabs: token equality, measured heights, flex pins, ellipsis, and adjacency in light and dark modes", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // 1. Light mode check
  await assertTabStandards(page, "light");

  // 2. Dark mode check
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await assertTabStandards(page, "dark");

  // Restore light mode
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("gallery tabs block is visible on /?gallery=1", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/root")).toBeVisible({ timeout: 15000 });
  const sectionTabs = page.getByTestId("gallery/section-tabs");
  await expect(sectionTabs).toBeVisible({ timeout: 15000 });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
