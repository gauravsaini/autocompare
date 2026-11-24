/**
 * Wave-33 App dirty-dot wiring proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (no harness) and proves the wave-31
 * dirty-dot + close-confirm lights up from real tab state, wired in
 * `App.tsx` only (per-tab last-run snapshot vs current compare inputs;
 * no new IPC, no layout changes):
 *  - fresh compare inputs (never run via the toolbar) mark the tab dirty;
 *  - toolbar Compare clears the dot on the run tab only (failed/blocked
 *    runs keep it); other tabs keep their dots (per-tab isolation);
 *  - switching tabs preserves each tab's dot;
 *  - closing the last tab shows the welcome tab with no dot and no close
 *    affordance; closing a dirty tab still arms the inline confirm
 *    (Cancel keeps, Confirm discards);
 *  - light + dark PNGs land in `/tmp/ui-33/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * No new IPC: every assertion reads the DOM. Skip policy matches the
 * other specs (infra failures skip). NEW file only; no existing spec
 * touched.
 *
 * Wave-74 collateral: the Home tab is now always present (welcome
 * two-pane stays reachable with sessions open), so tab counts below are
 * +1 vs the original wave-33 expectations and `tab-home` is visible
 * alongside session tabs (still never dirty, never closable).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-33";

interface TabState {
  id: string;
  title: string;
  dirty: boolean;
}

async function tabbarStates(page: Page): Promise<TabState[]> {
  return page.getByTestId("tabbar").evaluate((bar) => {
    const tabs = [...bar.querySelectorAll('[role="tab"]')];
    return tabs.map((t) => {
      const testid = t.getAttribute("data-testid") ?? "";
      const dirty = t.querySelector('[data-testid^="tab-dirty-"]') !== null;
      return {
        id: testid.replace(/^tab-/, ""),
        title: (t.textContent ?? "").trim(),
        dirty,
      };
    });
  });
}

async function dirtyCount(page: Page): Promise<number> {
  return page.getByTestId("tabbar").locator('[data-testid^="tab-dirty-"]').count();
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("app dirty dots: fresh inputs dirty, compare clears per-tab, switch preserves, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  const tabbar = page.getByTestId("tabbar");
  const compare = page.getByTestId("toolbar/action-compare");

  // --- demo session auto-boots with fresh (never-run) inputs: dirty dot ---
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1", {
    timeout: 15000,
  });
  await expect(tabbar.getByRole("tab")).toHaveCount(2, { timeout: 10000 });
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  let states = await tabbarStates(page);
  expect(states).toHaveLength(2);
  // Wave-74: [home, demo] — the home tab is never dirty.
  expect(states.find((s) => s.title.includes("t_left.txt"))?.dirty).toBe(true);
  expect(states.find((s) => s.id === "home")?.dirty).toBe(false);
  await expect(tabbar.getByTestId("tab-home")).toBeVisible();
  await expect(tabbar.getByTestId("tab-dirty-home")).toHaveCount(0);
  await expect(tabbar.getByTestId("tab-close-home")).toHaveCount(0);

  // --- toolbar Compare clears the run tab's dot (successful run) ---
  await expect(compare).toBeEnabled({ timeout: 15000 });
  await compare.click();
  await expect.poll(() => dirtyCount(page), { timeout: 15000 }).toBe(0);
  states = await tabbarStates(page);
  expect(states).toHaveLength(2);
  expect(states.find((s) => s.title.includes("t_left.txt"))?.dirty).toBe(false);

  // --- new compare inputs (second session) arrive dirty; first stays clean ---
  await page.getByTestId("toolbar/action-open").click();
  const dialog = page.getByTestId("session/dialog");
  await expect(dialog).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("text");
  await page.getByTestId("session/left-path").fill("/tmp/dirty-b-left.txt");
  await page.getByTestId("session/right-path").fill("/tmp/dirty-b-right.txt");
  await page.getByTestId("session/create-button").click();
  await expect(page.getByTestId("session/list")).toContainText(
    "text: dirty-b-left.txt ↔ dirty-b-right.txt",
  );
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 2", {
    timeout: 10000,
  });
  await expect(tabbar.getByRole("tab")).toHaveCount(3, { timeout: 10000 });
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  states = await tabbarStates(page);
  expect(states).toHaveLength(3);
  const fresh = states.find((s) => s.title.includes("dirty-b-left"));
  const cleaned = states.find((s) => s.title.includes("t_left.txt"));
  expect(fresh?.dirty).toBe(true);
  expect(cleaned?.dirty).toBe(false);

  // --- switching tabs preserves each tab's dot ---
  const cleanedTab = tabbar.getByRole("tab").filter({ hasText: "t_left.txt" });
  await cleanedTab.click();
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  states = await tabbarStates(page);
  expect(states.find((s) => s.title.includes("dirty-b-left"))?.dirty).toBe(true);
  expect(states.find((s) => s.title.includes("t_left.txt"))?.dirty).toBe(false);
  const freshTab = tabbar.getByRole("tab").filter({ hasText: "dirty-b-left" });
  await freshTab.click();
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  states = await tabbarStates(page);
  expect(states.find((s) => s.title.includes("dirty-b-left"))?.dirty).toBe(true);

  // --- Compare on the dirty tab clears it; the other tab was already clean ---
  await expect(compare).toBeEnabled({ timeout: 15000 });
  await compare.click();
  await expect.poll(() => dirtyCount(page), { timeout: 15000 }).toBe(0);

  // --- light + dark evidence (never in-repo) ---
  await page.screenshot({ path: `${EVID_DIR}/tabbar-dirty-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/tabbar-dirty-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("app dirty dots: dirty close confirms, welcome never dirty nor closable", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  const tabbar = page.getByTestId("tabbar");

  // --- demo tab boots dirty; closing it arms the inline confirm ---
  // Wave-74: [home, demo] while sessions exist; home has no close button.
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1", {
    timeout: 15000,
  });
  await expect(tabbar.getByRole("tab")).toHaveCount(2, { timeout: 10000 });
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  const demoTab = tabbar.getByRole("tab").filter({ hasText: "t_left.txt" });
  await demoTab.getByRole("button", { name: /^Close / }).click();
  const prompt = tabbar.locator('[data-testid^="tab-close-prompt-"]');
  await expect(prompt).toBeVisible({ timeout: 10000 });
  await expect(tabbar.getByRole("tab")).toHaveCount(2);
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);

  // --- Cancel keeps the dirty tab ---
  await tabbar.getByRole("button", { name: /^Keep .* open$/ }).click();
  await expect(prompt).toBeHidden();
  await expect(tabbar.getByRole("tab")).toHaveCount(2);
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(1);
  await page.screenshot({ path: `${EVID_DIR}/tabbar-dirty-confirm.png` });

  // --- Confirm discards; welcome tab is never dirty and never closable ---
  await demoTab.getByRole("button", { name: /^Close / }).click();
  await expect(prompt).toBeVisible({ timeout: 10000 });
  await tabbar.getByRole("button", { name: /^Discard changes and close / }).click();
  await expect(tabbar.getByTestId("tab-home")).toBeVisible({ timeout: 10000 });
  await expect(tabbar.getByRole("tab")).toHaveCount(1);
  await expect.poll(() => dirtyCount(page), { timeout: 10000 }).toBe(0);
  await expect(tabbar.locator('[data-testid^="tab-dirty-"]')).toHaveCount(0);
  await expect(tabbar.locator('[data-testid^="tab-close-home"]')).toHaveCount(0);
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
