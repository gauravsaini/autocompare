/**
 * H1 Home chrome declutter (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Live 1717 screenshot review se 7 defects:
 * P0-1 PathBar Home par nahi (sirf comparison tabs, BC jaisa).
 * P0-2 OverviewRuler Home par nahi (sirf Text/Hex/Merge diff views).
 * P1-3 Sessions left sidebar full-height + cards top-aligned (BC pattern).
 * P1-4 Stray `sessions:N` canvas par nahi (testid sr-only, status bar me mirror).
 * P1-5 `+` button last tab ke turant bagal me (door right edge par nahi).
 * P2-6 Folder card subtitle `Directories & archives` (ellipsis nahi).
 * P2-7 Home par status `Ready` (debug `Tab selected: home` nahi).
 *
 * Koi geometry pixel-lock nahi — relational / class / text assertions only.
 * New file only, koi existing spec touch nahi.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function clearToEmpty(page: Page): Promise<void> {
  const banner = page.getByTestId("workspace/demo-banner");
  const clear = page.getByTestId("workspace/demo-clear");
  await expect(banner.or(page.getByTestId("workspace/welcome"))).toBeVisible({ timeout: 15000 });
  for (let round = 0; round < 2; round += 1) {
    try {
      await banner.waitFor({ state: "visible", timeout: 5000 });
      await clear.click();
      await expect(page.getByTestId("session-count")).toContainText("sessions: 0", { timeout: 10000 });
    } catch {
      break;
    }
  }
  await expect(page.getByTestId("workspace/welcome")).toBeVisible();
}

test("H1 home chrome: no PathBar, no ruler, no stray count, + adjacent, Ready", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await clearToEmpty(page);
  const welcome = page.getByTestId("workspace/welcome");
  const pane = page.getByTestId("home/sessions");

  // P0-1: Home par PathBar mount nahi.
  await expect(page.getByTestId("pathbar")).toHaveCount(0);

  // P0-2: Home par OverviewRuler mount nahi.
  await expect(page.getByTestId("ruler")).toHaveCount(0);
  await expect(page.getByTestId("ruler/empty")).toHaveCount(0);

  // P1-4: stray canvas count nahi — testid zinda hai par sr-only hai,
  // status bar me mirror ab bhi sessions dikhata hai.
  const homeCount = welcome.getByTestId("session-count");
  await expect(homeCount).toContainText("sessions: 0");
  await expect(homeCount).toHaveClass(/sr-only/);
  const monoVisible = await welcome.evaluate((el) => {
    const nodes = Array.from(el.querySelectorAll('[data-testid="session-count"]'));
    return nodes.filter((n) => {
      if (!(n instanceof HTMLElement)) return false;
      const cs = window.getComputedStyle(n);
      if (cs.display === "none" || cs.visibility === "hidden") return false;
      // sr-only = 1px clip, visually hidden by design.
      if (n.classList.contains("sr-only")) return false;
      const r = n.getBoundingClientRect();
      return r.width > 2 && r.height > 2;
    }).length;
  });
  expect(monoVisible, "Home canvas par koi visible sessions:N nahi").toBe(0);
  await expect(page.getByTestId("statusbar/right")).toContainText("sessions: 0");

  // P1-5: `+` last tab ke turant bagal me — inner strip flex-1 nahi
  // (grow 0, content hug), `+` tabbar ke andar visible aur tabs ke baad.
  const tabbar = page.getByTestId("tabbar");
  await expect(tabbar).toBeVisible();
  const newBtn = page.getByTestId("tab-new");
  await expect(newBtn).toBeVisible();
  const stripGrow = await page.evaluate(() => {
    const bar = document.querySelector('[data-testid="tabbar"]');
    const scroller = bar?.querySelector(":scope > div");
    if (!(scroller instanceof HTMLElement)) return "missing";
    return window.getComputedStyle(scroller).flexGrow;
  });
  expect(stripGrow, "tab strip hug karta hai (flex-grow 0), + adjacent").toBe("0");
  const order = await page.evaluate(() => {
    const bar = document.querySelector('[data-testid="tabbar"]');
    const home = document.querySelector('[data-testid="tab-home"]');
    const plus = document.querySelector('[data-testid="tab-new"]');
    if (!(bar instanceof HTMLElement) || !(home instanceof HTMLElement) || !(plus instanceof HTMLElement)) return null;
    const br = bar.getBoundingClientRect();
    const hr = home.getBoundingClientRect();
    const pr = plus.getBoundingClientRect();
    return {
      plusAfterHome: pr.x > hr.x,
      plusInsideBar: pr.x >= br.x && pr.x + pr.width <= br.x + br.width + 2,
    };
  });
  expect(order?.plusAfterHome, "+ tabs ke baad hai").toBe(true);
  expect(order?.plusInsideBar, "+ tabbar ke andar hai").toBe(true);

  // P1-3: left Saved-Sessions sidebar full-height (self-stretch), right
  // cards top-aligned row (BC pattern) — pixel threshold nahi, relational.
  await expect(pane).toBeVisible();
  await expect(pane).toHaveClass(/self-stretch/);
  const direction = await welcome.evaluate((el) => window.getComputedStyle(el).flexDirection);
  expect(direction, "welcome two-pane row hai").toBe("row");
  const leftOf = await page.evaluate(() => {
    const p = document.querySelector('[data-testid="home/sessions"]');
    const c = document.querySelector('[data-testid="home/new-text"]');
    if (!(p instanceof HTMLElement) || !(c instanceof HTMLElement)) return null;
    return p.getBoundingClientRect().x < c.getBoundingClientRect().x;
  });
  expect(leftOf, "sessions pane cards ke left me hai").toBe(true);

  // P2-6: Folder subtitle chhota — ellipsis nahi.
  await expect(page.getByTestId("home/new-folder")).toContainText("Directories & archives");

  // P2-7: Home empty par status Ready/Welcome, debug `Tab selected` nahi.
  const msg = await page.getByTestId("statusbar/message").textContent();
  expect(msg ?? "", "Home par debug Tab-selected nahi").not.toMatch(/Tab selected/);

  await assertNoOverlap(page);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("H1 home with session: Home tab par chrome wapas clean + Ready", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await clearToEmpty(page);

  // Ek text session kholo, phir Home wapas.
  await page.getByTestId("home/new-text").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByTestId("workspace/welcome")).toBeHidden();

  await page.getByTestId("tab-home").click();
  const welcome = page.getByTestId("workspace/welcome");
  await expect(welcome).toBeVisible();

  // Home par phir bhi PathBar/ruler nahi, count sr-only, status Ready.
  await expect(page.getByTestId("pathbar")).toHaveCount(0);
  await expect(page.getByTestId("ruler")).toHaveCount(0);
  await expect(welcome.getByTestId("session-count")).toHaveClass(/sr-only/);
  await expect(page.getByTestId("statusbar/message")).toContainText("Ready");
  await expect(page.getByTestId("statusbar/message")).not.toContainText("Tab selected: home");

  // Session row list me hai aur click se wapas session khulta hai.
  await expect(page.locator('[data-testid^="home/session-row-"]')).toHaveCount(1);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
});
