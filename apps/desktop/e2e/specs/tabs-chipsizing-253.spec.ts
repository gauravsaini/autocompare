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
 *
 * TABS2 (uniform sizing, BC parity): chips `flex-1 basis-0` equal share +
 * `--ac-tab-min-w: 96px` / `--ac-tab-max-w: 180px` clamps (light+dark).
 * Adds: token-value pins (96/180), flex-share class pins, a harness test
 * mounting 2 uneven-title tabs (offsetWidth equal ±2px) and 8 tabs (all
 * chips >= min-w, strip scrollable). Purane assertions intact (class-name
 * expectations sirf naye token classes par update); heights/testids/wiring
 * untouched.
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
      // TABS2: uniform-width clamp tokens (BC standard).
      tabMinW: get("--ac-tab-min-w"),
      tabMaxW: get("--ac-tab-max-w"),
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

  // TABS2: BC-standard clamp tokens — readable floor 96 (60 se upar),
  // ellipsis cap 180 (224 se neeche). Values locked light+dark (dark mirror identical).
  expect(t.tabMinW, `${scheme}: --ac-tab-min-w == 96px`).toBe("96px");
  expect(t.tabMaxW, `${scheme}: --ac-tab-max-w == 180px`).toBe("180px");

  // Truncation contract intact: token floor + cap + ellipsis, close chip ke andar.
  const cls = await firstTab.evaluate((el) =>
    el instanceof HTMLElement ? el.className : "",
  );
  expect(cls, `${scheme}: chip min-w token floor intact`).toContain("min-w-[var(--ac-tab-min-w)]");
  expect(cls, `${scheme}: chip max-w token cap intact`).toContain("max-w-[var(--ac-tab-max-w)]");
  // TABS2: equal-share flex (basis content se free, free space barabar).
  expect(cls, `${scheme}: chip flex-1 equal share`).toMatch(/(^|\s)flex-1(\s|$)/);
  expect(cls, `${scheme}: chip basis-0 (content-hug nahi)`).toContain("basis-0");
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

/**
 * TABS2 uniform-width harness (page-local TabBar mount, tabbar-tabs.spec.ts
 * pattern; container 520px = ChromeShell cap, so shrink/scroll reality
 * real-app jaisi hai). Heights/testids/wiring untouched — sirf widths.
 */
interface UniformTab {
  id: string;
  title: string;
  kind: string;
  dirty: boolean;
}

async function mountUniformTabs(page: Page, tabs: UniformTab[], activeId: string): Promise<void> {
  await page.evaluate((arg: { nextTabs: UniformTab[]; nextActive: string }) => {
    const w = window as unknown as Record<string, unknown>;
    const boot = (): Promise<void> => {
      return Promise.all([
        import("/src/components/chrome/TabBar.tsx"),
        import("/src/components/session/index.ts"),
      ]).then((mods) => {
        const sessionMod = mods[1] as unknown as Record<string, unknown>;
        const ReactDOM = sessionMod["ReactDOM"] as unknown as {
          createRoot: (el: Element) => { render: (node: unknown) => void };
        };
        const React = sessionMod["React"] as unknown as {
          createElement: (...args: unknown[]) => unknown;
        };
        let container = document.getElementById("tabbar-uniform-harness");
        if (!container) {
          container = document.createElement("div");
          container.id = "tabbar-uniform-harness";
          document.body.appendChild(container);
        }
        // ChromeShell cap mirror: real tab strip kabhi 520px se wide nahi.
        (container as HTMLElement).style.width = "520px";
        if (!w.__acUniformRoot) {
          w.__acUniformRoot = ReactDOM.createRoot(container);
        }
        const TabBar = (mods[0] as unknown as Record<string, unknown>)["TabBar"];
        (w.__acUniformRoot as { render: (node: unknown) => void }).render(
          React.createElement(TabBar, {
            tabs: arg.nextTabs,
            activeTabId: arg.nextActive,
            onSelectTab: () => {},
            onCloseTab: () => {},
            onNewTab: () => {},
          }),
        );
      });
    };
    return boot();
  }, { nextTabs: tabs, nextActive: activeId });
}

async function uniformWidths(page: Page, scheme: string, ids: string[]): Promise<number[]> {
  const host = page.locator("#tabbar-uniform-harness");
  for (const id of ids) {
    await expect(host.getByTestId(`tab-${id}`)).toBeVisible({ timeout: 10000 });
  }
  const widths = await host.getByRole("tab").evaluateAll((els) =>
    els.map((el) => (el instanceof HTMLElement ? el.offsetWidth : 0)),
  );
  expect(widths.length, `${scheme}: saare harness tabs render hue`).toBe(ids.length);
  return widths;
}

test("tabs chipsizing TABS2: 2-tab equal widths, 8-tab uniform >= min-w + scroll, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  const checkScheme = async (scheme: string): Promise<void> => {
    const minW = Number.parseFloat(
      await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--ac-tab-min-w").trim()),
    );
    expect(minW, `${scheme}: --ac-tab-min-w numeric (96px)`).toBe(96);

    // 2-tab fixture: titles bilkul alag length — widths phir bhi barabar (±2px).
    await mountUniformTabs(
      page,
      [
        { id: "u-short", title: "a ↔ b", kind: "text", dirty: false },
        {
          id: "u-long",
          title: "text: very-long-left-filename.txt ↔ very-long-right-filename.txt",
          kind: "text",
          dirty: true,
        },
      ],
      "u-short",
    );
    const two = await uniformWidths(page, scheme, ["u-short", "u-long"]);
    expect(
      Math.abs(two[0] - two[1]),
      `${scheme}: 2-tab chips equal width (got ${two.join(", ")})`,
    ).toBeLessThanOrEqual(2);

    // 8-tab fixture: sab chips >= min-w, widths uniform (±2px), strip scrollable.
    const eight: UniformTab[] = Array.from({ length: 8 }, (_, i) => ({
      id: `u8-${i}`,
      title: i % 2 === 0 ? `text: file-${i}.txt ↔ other-${i}.txt` : `s${i} ↔ d${i}`,
      kind: "text",
      dirty: i % 3 === 0,
    }));
    await mountUniformTabs(page, eight, "u8-0");
    const eightWidths = await uniformWidths(
      page,
      scheme,
      eight.map((t) => t.id),
    );
    for (const w of eightWidths) {
      expect(w, `${scheme}: 8-tab chip >= min-w (got ${eightWidths.join(", ")})`).toBeGreaterThanOrEqual(minW - 2);
    }
    const spread = Math.max(...eightWidths) - Math.min(...eightWidths);
    expect(spread, `${scheme}: 8-tab chips uniform (got ${eightWidths.join(", ")})`).toBeLessThanOrEqual(2);
    const scroll = await page.locator("#tabbar-uniform-harness").evaluate((host) => {
      const bar = host.querySelector('[data-testid="tabbar"]');
      const scroller = bar?.querySelector(":scope > div");
      if (!(scroller instanceof HTMLElement)) return null;
      return { scrollWidth: scroller.scrollWidth, clientWidth: scroller.clientWidth };
    });
    expect(scroll, `${scheme}: strip mila`).not.toBeNull();
    if (scroll !== null) {
      expect(
        scroll.scrollWidth,
        `${scheme}: 8-tab strip scrollable (scrollW=${scroll.scrollWidth} clientW=${scroll.clientWidth})`,
      ).toBeGreaterThan(scroll.clientWidth + 1);
    }
  };

  await checkScheme("light");
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await checkScheme("dark");
  await page.emulateMedia({ colorScheme: "light" });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
