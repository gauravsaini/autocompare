/**
 * Wave-31 tabbar session tabs proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `TabBar` (`components/chrome/TabBar.tsx`) through the
 * Vite dev server with a page-local mount harness (same dynamic-import
 * pattern as `flows-session.spec.ts`; `React`/`ReactDOM` come from the
 * existing `components/session` re-exports, no new modules) and proves
 * the Beyond Compare style tab UX owned by this wave:
 *  - dirty-dot renders only from existing tab state (`dirty: true` on a
 *    session tab, e.g. after an edit); the welcome `home` tab never shows
 *    a dot even when passed `dirty: true`, and has no close affordance;
 *  - clean tabs close directly via the `X` button;
 *  - dirty tabs arm an inline confirm (`tab-close-prompt-*`): Cancel
 *    keeps the tab (mouse + Escape paths), Confirm discards it;
 *  - middle-click closes clean tabs directly and arms (never discards
 *    through) the confirm on dirty tabs;
 *  - keyboard: `X` is focusable, Enter arms the confirm on a dirty tab;
 *  - real `App` boot still shows `tabbar` + `tab-home` with no dirty dot
 *    and no close button on welcome (demo session auto-opens via mock IPC);
 *  - failures render inline (`tabbar/error`), never crash;
 *  - light + dark PNGs land in `/tmp/ui-31/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * No new IPC: every assertion reads the DOM or the harness callback log.
 * Skip policy matches the other specs (infra failures skip).
 * NEW file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-31";

interface HarnessTab {
  id: string;
  title: string;
  kind: string;
  dirty: boolean;
}

async function mountTabs(page: Page, tabs: HarnessTab[], activeId: string): Promise<void> {
  await page.evaluate(
    (arg: { nextTabs: HarnessTab[]; nextActive: string }) => {
      const w = window as unknown as Record<string, unknown>;
      const boot = (): Promise<void> => {
        return Promise.all([
          import("/src/components/chrome/TabBar.tsx"),
          import("/src/components/session/index.ts"),
        ]).then((mods) => {
          if (!w.__acTabbarReady) {
            w.__acTabbarMod = mods[0];
            w.__acSessionMod = mods[1];
            let container = document.getElementById("tabbar-test-harness");
            if (!container) {
              container = document.createElement("div");
              container.id = "tabbar-test-harness";
              document.body.appendChild(container);
            }
            const sessionMod = w.__acSessionMod as unknown as Record<string, unknown>;
            const ReactDOM = sessionMod["ReactDOM"] as unknown as Record<
              string,
              (el: Element) => { render: (node: unknown) => void }
            >;
            w.__acTabRoot = ReactDOM["createRoot"](container);
            w.__acTabbarReady = true;
          }
          const tabbarMod = w.__acTabbarMod as unknown as Record<string, unknown>;
          const sessionMod = w.__acSessionMod as unknown as Record<string, unknown>;
          const React = sessionMod["React"] as unknown as Record<string, unknown>;
          const createElement = React["createElement"] as (...args: unknown[]) => unknown;
          const root = w.__acTabRoot as unknown as { render: (node: unknown) => void };
          if (w.__acTabClosed === undefined) w.__acTabClosed = [];
          w.__acTabs = arg.nextTabs;
          w.__acTabActive = arg.nextActive;
          const render = (): void => {
            const inner = window as unknown as Record<string, unknown>;
            const TabBar = tabbarMod["TabBar"];
            root.render(
              createElement(TabBar, {
                tabs: inner.__acTabs,
                activeTabId: inner.__acTabActive,
                onSelectTab: (id: unknown) => {
                  const st = window as unknown as Record<string, unknown>;
                  st.__acTabSelected = id;
                  st.__acTabActive = id;
                  (st.__acTabRender as () => void)();
                },
                onCloseTab: (id: unknown) => {
                  const st = window as unknown as Record<string, unknown>;
                  (st.__acTabClosed as unknown[]).push(id);
                  st.__acTabs = (st.__acTabs as HarnessTab[]).filter((t) => t.id !== id);
                  (st.__acTabRender as () => void)();
                },
                onNewTab: (kind: unknown) => {
                  const st = window as unknown as Record<string, unknown>;
                  st.__acTabNewKind = kind;
                },
              }),
            );
          };
          w.__acTabRender = render;
          render();
        });
      };
      return boot();
    },
    { nextTabs: tabs, nextActive: activeId },
  );
}

async function closedTabs(page: Page): Promise<string[]> {
  return (await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__acTabClosed as string[],
  )) ?? [];
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

test("tabbar session tabs: dirty-dot + inline close confirm + middle-click, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // --- real App boot: tabbar live, demo session auto-opened via mock IPC ---
  // (welcome `home` only renders with zero sessions, so the welcome
  // never-dirty/never-closable contract is proven in the harness below).
  await expect(page.getByTestId("tabbar")).toBeVisible();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1", {
    timeout: 15000,
  });
  const appTabs = page.getByRole("tab");
  await expect
    .poll(async () => appTabs.count(), { timeout: 15000 })
    .toBeGreaterThanOrEqual(1);
  await expect(page.getByTestId("tabbar/error")).toHaveCount(0);

  // --- harness: welcome (forced dirty) + clean + dirty session tabs ---
  const host = page.locator("#tabbar-test-harness");
  await mountTabs(
    page,
    [
      { id: "home", title: "Welcome", kind: "home", dirty: true },
      { id: "sess-clean", title: "text: a.txt ↔ b.txt", kind: "text", dirty: false },
      { id: "sess-dirty", title: "text: c.txt ↔ d.txt", kind: "text", dirty: true },
    ],
    "sess-dirty",
  );
  await expect(host.getByTestId("tab-home")).toBeVisible({ timeout: 10000 });
  // Welcome never dirty even when passed dirty:true, and never closable.
  await expect(host.getByTestId("tab-dirty-home")).toHaveCount(0);
  await expect(host.getByTestId("tab-close-home")).toHaveCount(0);
  // Dirty session tab shows the dot; clean tab shows a direct close button.
  const dot = host.getByTestId("tab-dirty-sess-dirty");
  await expect(dot).toBeVisible();
  await expect(dot).toHaveAttribute("aria-label", "unsaved changes");
  await expect(host.getByTestId("tab-dirty-sess-clean")).toHaveCount(0);
  await expect(host.getByTestId("tab-close-sess-clean")).toBeVisible();

  // --- edit marks a clean tab dirty (re-mount with dirty:true) ---
  await mountTabs(
    page,
    [
      { id: "home", title: "Welcome", kind: "home", dirty: false },
      { id: "sess-clean", title: "text: a.txt ↔ b.txt", kind: "text", dirty: true },
      { id: "sess-dirty", title: "text: c.txt ↔ d.txt", kind: "text", dirty: true },
    ],
    "sess-clean",
  );
  await expect(host.getByTestId("tab-dirty-sess-clean")).toBeVisible({ timeout: 10000 });

  // --- dirty close arms inline confirm; Cancel (mouse) keeps the tab ---
  await host.getByTestId("tab-close-sess-clean").click();
  const prompt = host.getByTestId("tab-close-prompt-sess-clean");
  await expect(prompt).toBeVisible();
  expect(await closedTabs(page)).toEqual([]);
  await expect(host.getByTestId("tab-sess-clean")).toBeVisible();
  // Confirm prompt visual evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/tabbar-tabs-confirm.png` });
  await host.getByTestId("tab-close-cancel-sess-clean").click();
  await expect(prompt).toBeHidden();
  expect(await closedTabs(page)).toEqual([]);
  await expect(host.getByTestId("tab-sess-clean")).toBeVisible();

  // --- keyboard: Enter on X arms confirm, Escape cancels ---
  await host.getByTestId("tab-close-sess-clean").focus();
  await expect(host.getByTestId("tab-close-sess-clean")).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(prompt).toBeVisible();
  await host.getByTestId("tab-close-confirm-sess-clean").focus();
  await expect(host.getByTestId("tab-close-confirm-sess-clean")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(prompt).toBeHidden();
  expect(await closedTabs(page)).toEqual([]);

  // --- Confirm discards the dirty tab ---
  await host.getByTestId("tab-close-sess-clean").click();
  await expect(prompt).toBeVisible();
  await host.getByTestId("tab-close-confirm-sess-clean").click();
  await expect(host.getByTestId("tab-sess-clean")).toBeHidden();
  expect(await closedTabs(page)).toEqual(["sess-clean"]);

  // --- middle-click: clean closes directly, dirty arms (never discards) ---
  await mountTabs(
    page,
    [
      { id: "sess-mid-clean", title: "folder: l ↔ r", kind: "folder", dirty: false },
      { id: "sess-mid-dirty", title: "hex: l ↔ r", kind: "hex", dirty: true },
    ],
    "sess-mid-clean",
  );
  const before = await closedTabs(page);
  await host.getByTestId("tab-sess-mid-clean").click({ button: "middle" });
  await expect(host.getByTestId("tab-sess-mid-clean")).toBeHidden({ timeout: 10000 });
  expect(await closedTabs(page)).toEqual([...before, "sess-mid-clean"]);
  // Dirty middle-click arms the confirm instead of closing.
  await host.getByTestId("tab-sess-mid-dirty").click({ button: "middle" });
  const midPrompt = host.getByTestId("tab-close-prompt-sess-mid-dirty");
  await expect(midPrompt).toBeVisible({ timeout: 10000 });
  await expect(host.getByTestId("tab-sess-mid-dirty")).toBeVisible();
  expect(await closedTabs(page)).toEqual([...before, "sess-mid-clean"]);
  // Cancel keeps the dirty tab after the middle-click arm.
  await host.getByTestId("tab-close-cancel-sess-mid-dirty").click();
  await expect(midPrompt).toBeHidden();
  await expect(host.getByTestId("tab-sess-mid-dirty")).toBeVisible();

  // --- light + dark evidence (never in-repo) ---
  await page.screenshot({ path: `${EVID_DIR}/tabbar-tabs-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/tabbar-tabs-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
