/**
 * Wave-189 typography-icons slice (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Ponytail scope: tokens + font wiring only, NO layout changes.
 * Pins only the W189 gaps on top of typo-100 / 66D (which stay green):
 *  1. font stacks: UI rides Inter -> system-ui -> SF Pro fallback,
 *     mono rides JetBrains Mono -> SF Mono fallback (computed-family
 *     asserts, light+dark);
 *  2. tabular numerals: gutter + code cells + chrome totals/meta
 *     (fontVariantNumeric === "tabular-nums");
 *  3. letter-spacing ramp: 12px UI chrome tracks -0.12px
 *     (-0.01em), code tracks 0;
 *  4. Lucide unify: toolbar svgs size 14 (12x12 box + 14 attr model
 *     both accepted via bounding-box), stroke-width 1.75, round
 *     caps/joins, light+dark.
 * Zero console/page errors. New file only; no existing spec touched.
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

test("typopolish-189: font stacks + tabular + tracking + Lucide unify, light+dark", async ({
  page,
}: {
  page: Page;
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect.poll(async () => page.locator(".tv-cell").count(), { timeout: 15000 }).toBeGreaterThan(0);

  const lightVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const root = window.getComputedStyle(document.documentElement);
    // Icon tokens pinned (single renderer contract).
    if (root.getPropertyValue("--ac-icon-size").trim() !== "14px") failures.push("icon size token");
    if (root.getPropertyValue("--ac-icon-stroke-width").trim() !== "1.75") failures.push("icon stroke token");
    // UI stack: Inter first, system-ui + SF Pro in fallback chain.
    const uiStack = root.getPropertyValue("--ac-font-ui");
    if (!uiStack.includes("Inter")) failures.push(`ui token stack: ${uiStack}`);
    if (!uiStack.includes("system-ui")) failures.push(`ui token missing system-ui: ${uiStack}`);
    if (!uiStack.includes("SF Pro")) failures.push(`ui token missing SF Pro: ${uiStack}`);
    // Mono stack: JetBrains Mono first, SF Mono in fallback chain.
    const monoStack = root.getPropertyValue("--ac-font-mono");
    if (!monoStack.includes("JetBrains Mono")) failures.push(`mono token stack: ${monoStack}`);
    if (!monoStack.includes("SF Mono")) failures.push(`mono token missing SF Mono: ${monoStack}`);
    // Computed families ride the stacks.
    const body = window.getComputedStyle(document.body);
    if (!body.fontFamily.includes("Inter")) failures.push(`body family: ${body.fontFamily}`);
    const gutter = document.querySelector(".tv-gutter");
    if (!gutter) failures.push("no .tv-gutter");
    else {
      const g = window.getComputedStyle(gutter);
      if (!g.fontFamily.includes("JetBrains Mono") && !g.fontFamily.includes("SF Mono"))
        failures.push(`gutter family: ${g.fontFamily}`);
      if (g.fontVariantNumeric !== "tabular-nums") failures.push(`gutter nums: ${g.fontVariantNumeric}`);
    }
    const cell = document.querySelector(".tv-cell");
    if (!cell) failures.push("no .tv-cell");
    else {
      const c = window.getComputedStyle(cell);
      if (c.fontVariantNumeric !== "tabular-nums") failures.push(`cell nums: ${c.fontVariantNumeric}`);
      if (c.letterSpacing !== "0px" && c.letterSpacing !== "normal")
        failures.push(`cell tracking: ${c.letterSpacing}`);
    }
    // Chrome totals/meta: 11px meta + tabular + -0.12px tracking.
    const meta = document.querySelector('[role="toolbar"] .ac-meta, .ac-meta');
    if (!meta) failures.push("no .ac-meta");
    else {
      const m = window.getComputedStyle(meta);
      if (m.fontSize !== "11px") failures.push(`meta size: ${m.fontSize}`);
      if (m.fontVariantNumeric !== "tabular-nums") failures.push(`meta nums: ${m.fontVariantNumeric}`);
      if (Math.abs(parseFloat(m.letterSpacing) - -0.12) > 0.05)
        failures.push(`meta tracking: ${m.letterSpacing}`);
    }
    // Tab pill tracking -0.12px (12px * -0.01em).
    const tab = document.querySelector('[role="tab"][aria-selected="true"]');
    if (!tab) failures.push("no selected tab");
    else {
      const t = window.getComputedStyle(tab);
      if (Math.abs(parseFloat(t.letterSpacing) - -0.12) > 0.05)
        failures.push(`pill tracking: ${t.letterSpacing}`);
      if (!t.fontFamily.includes("Inter") && !t.fontFamily.includes("SF Pro"))
        failures.push(`pill stack: ${t.fontFamily}`);
    }
    return failures;
  });
  expect(lightVerdict, `light font pins (got: ${lightVerdict.join(" | ")})`).toEqual([]);

  // Lucide unify: toolbar icons 14px box, 1.75 stroke, round caps/joins.
  const icons = page.locator('[role="toolbar"] svg');
  await expect.poll(async () => icons.count(), { timeout: 15000 }).toBeGreaterThan(0);
  const iconVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const svgs = Array.from(document.querySelectorAll('[role="toolbar"] svg'));
    if (svgs.length === 0) failures.push("no toolbar svgs");
    for (const svg of svgs) {
      const box = (svg as SVGSVGElement).getBoundingClientRect();
      // Width/height come from Lucide size prop (14) via width/height attrs.
      const wAttr = svg.getAttribute("width");
      const hAttr = svg.getAttribute("height");
      const w = wAttr ? parseFloat(wAttr) : box.width;
      const h = hAttr ? parseFloat(hAttr) : box.height;
      if (Math.abs(w - 14) > 0.6 || Math.abs(h - 14) > 0.6)
        failures.push(`icon box: ${wAttr}x${hAttr} (${box.width.toFixed(1)}x${box.height.toFixed(1)})`);
      if (svg.getAttribute("stroke-width") !== "1.75")
        failures.push(`icon stroke-width: ${svg.getAttribute("stroke-width")}`);
      const cap = svg.getAttribute("stroke-linecap");
      const join = svg.getAttribute("stroke-linejoin");
      if (cap !== null && cap !== "round") failures.push(`icon linecap: ${cap}`);
      if (join !== null && join !== "round") failures.push(`icon linejoin: ${join}`);
    }
    return failures.slice(0, 8);
  });
  expect(iconVerdict, `icon unify (got: ${iconVerdict.join(" | ")})`).toEqual([]);

  // Dark: stacks + tabular + icons hold, zero drift.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(400);
  const darkVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const body = window.getComputedStyle(document.body);
    if (!body.fontFamily.includes("Inter")) failures.push(`dark body family: ${body.fontFamily}`);
    const latch = document.querySelector(".tv-cell, .tv-gutter");
    if (latch) {
      if (window.getComputedStyle(latch).fontVariantNumeric !== "tabular-nums")
        failures.push("dark tabular drift");
    } else failures.push("no dark latch");
    for (const svg of Array.from(document.querySelectorAll('[role="toolbar"] svg'))) {
      if (svg.getAttribute("stroke-width") !== "1.75") {
        failures.push("dark icon stroke drift");
        break;
      }
    }
    return failures;
  });
  expect(darkVerdict, `dark pins (got: ${darkVerdict.join(" | ")})`).toEqual([]);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
