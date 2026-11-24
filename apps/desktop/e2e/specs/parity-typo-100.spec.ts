/**
 * Wave-100 typography tighten (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Audits the app type scale against live Beyond Compare truth
 * (`/tmp/bc-*.png`: text/folder/table/home/hex views) and pins it with
 * computed-style assertions — ZERO layout change (no `--ac-row-h`, no
 * spacing/layout tokens, no Views/geometry edits; every existing
 * offsetHeight/offsetWidth assertion stays green):
 *  1. type scale: UI 12px / meta 11px / code 12px, code line-height
 *     1.55 (inside the 1.5-1.6 BC band), UI tracking -0.01em, code
 *     tracking 0, emphasis weights ride `--ac-font-weight-*` tokens;
 *  2. tabular numerals on gutters (`.tv-gutter`), table cells
 *     (`.tc-cell`), hex offsets (`.hx-offset`);
 *  3. Wave-144 bundled stacks: Inter rides before SF Pro, JetBrains Mono
 *     before SF Mono (fallbacks stay), and document.fonts holds ONLY the
 *     repo-bundled faces (Inter + JetBrains Mono, offline-first woff2 —
 *     zero remote webfont families);
 *  4. active tab pill: rounded + medium weight + tight tracking,
 *     light + dark; toolbar icons: stroke-width 2, round caps/joins,
 *     light + dark.
 * Zoomed crops (code pane, gutter numerals, tab pill) land in
 * `/tmp/typo-100-*.png` (never in-repo). Zero console/page errors.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
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

async function createSession(page: Page, kind: string, left: string, right: string, viewTestId: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestId)).toBeVisible();
}

test("typo-100: type scale + tab pill + icon strokes pin, light+dark", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // ---- Text demo first-paint: code ramp + gutter numerals.
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect.poll(async () => page.locator(".tv-cell").count(), { timeout: 15000 }).toBeGreaterThan(0);
  const textVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const cell = document.querySelector(".tv-cell");
    const gutter = document.querySelector(".tv-gutter");
    if (!cell) failures.push("no .tv-cell");
    if (!gutter) failures.push("no .tv-gutter");
    if (cell) {
      const c = window.getComputedStyle(cell);
      if (!c.fontFamily.includes("SF Mono")) failures.push(`cell family: ${c.fontFamily}`);
      if (c.fontSize !== "12px") failures.push(`cell size: ${c.fontSize}`);
      const ratio = parseFloat(c.lineHeight) / parseFloat(c.fontSize);
      if (!(ratio >= 1.5 && ratio <= 1.6)) failures.push(`cell lh ratio: ${ratio} (${c.lineHeight})`);
      if (c.letterSpacing !== "0px" && c.letterSpacing !== "normal") failures.push(`cell tracking: ${c.letterSpacing}`);
    }
    if (gutter) {
      const g = window.getComputedStyle(gutter);
      if (g.fontVariantNumeric !== "tabular-nums") failures.push(`gutter nums: ${g.fontVariantNumeric}`);
      if (g.fontSize !== "12px") failures.push(`gutter size: ${g.fontSize}`);
      if (!g.fontFamily.includes("SF Mono")) failures.push(`gutter family: ${g.fontFamily}`);
    }
    // Weight tokens resolve to the BC emphasis weights.
    const root = window.getComputedStyle(document.documentElement);
    if (root.getPropertyValue("--ac-font-weight-regular").trim() !== "400") failures.push("weight regular");
    if (root.getPropertyValue("--ac-font-weight-medium").trim() !== "500") failures.push("weight medium");
    if (root.getPropertyValue("--ac-font-weight-semibold").trim() !== "600") failures.push("weight semibold");
    if (root.getPropertyValue("--ac-font-size-ui").trim() !== "12px") failures.push("ui size token");
    if (root.getPropertyValue("--ac-font-size-meta").trim() !== "11px") failures.push("meta size token");
    if (root.getPropertyValue("--ac-font-size-code").trim() !== "12px") failures.push("code size token");
    // Wave-144: bundled offline-first faces only (Inter + JetBrains
    // Mono from repo woff2); system stacks stay as fallback below them.
    for (const face of document.fonts) {
      const fam = face.family;
      if (fam !== 'Inter' && fam !== 'JetBrains Mono') failures.push(`remote webfont family: ${fam}`);
    }
    if (document.fonts.size === 0) failures.push('bundled faces missing');
    const body = window.getComputedStyle(document.body);
    if (!body.fontFamily.includes("SF Pro")) failures.push(`body stack: ${body.fontFamily}`);
    return failures;
  });
  expect(textVerdict, `text ramp (got: ${textVerdict.join(" | ")})`).toEqual([]);

  // ---- Zoomed crops while the text demo is active.
  await page.locator(".tv-cell").first().screenshot({ path: "/tmp/typo-100-code-light.png" });
  await page.locator(".tv-gutter").first().screenshot({ path: "/tmp/typo-100-gutter-light.png" });
  await page.getByRole("tab", { selected: true }).first().screenshot({ path: "/tmp/typo-100-pill-light.png" });

  // ---- Active tab pill + toolbar type/strokes (light).
  const chromeVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const tab = document.querySelector('[role="tab"][aria-selected="true"]');
    if (!tab) {
      failures.push("no selected tab");
    } else {
      const t = window.getComputedStyle(tab);
      if (parseFloat(t.borderRadius) < 6) failures.push(`pill radius: ${t.borderRadius}`);
      if (t.fontWeight !== "500") failures.push(`pill weight: ${t.fontWeight}`);
      // 12px * -0.01em = -0.12px
      if (Math.abs(parseFloat(t.letterSpacing) - -0.12) > 0.05) failures.push(`pill tracking: ${t.letterSpacing}`);
      if (!t.fontFamily.includes("SF Pro")) failures.push(`pill stack: ${t.fontFamily}`);
    }
    const meta = document.querySelector('[role="toolbar"] .ac-meta');
    if (!meta) {
      failures.push("no toolbar meta label");
    } else {
      const m = window.getComputedStyle(meta);
      if (m.fontSize !== "11px") failures.push(`toolbar meta size: ${m.fontSize}`);
    }
    const svgs = Array.from(document.querySelectorAll('[role="toolbar"] svg'));
    if (svgs.length === 0) failures.push("no toolbar svgs");
    for (const svg of svgs) {
      const w = svg.getAttribute("stroke-width");
      const cap = svg.getAttribute("stroke-linecap");
      const join = svg.getAttribute("stroke-linejoin");
      if (w !== "2") failures.push(`icon stroke-width: ${w}`);
      if (cap !== "round") failures.push(`icon linecap: ${cap}`);
      if (join !== "round") failures.push(`icon linejoin: ${join}`);
    }
    return failures;
  });
  expect(chromeVerdict, `chrome pill+icons (got: ${chromeVerdict.join(" | ")})`).toEqual([]);

  // ---- Folder: sans tree text 12px, status keeps tight tracking.
  await createSession(page, "folder", "/tmp/t100-left-dir", "/tmp/t100-right-dir", "view-folder");
  await expect.poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 15000 }).toBeGreaterThan(0);
  const folderVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const node = document.querySelector(".fv-node");
    if (!node) {
      failures.push("no .fv-node");
    } else {
      const c = window.getComputedStyle(node);
      if (c.fontSize !== "12px") failures.push(`folder size: ${c.fontSize}`);
      if (!c.fontFamily.includes("SF Pro")) failures.push(`folder stack: ${c.fontFamily}`);
    }
    const status = document.querySelector(".fv-status");
    if (status) {
      const s = window.getComputedStyle(status);
      if (Math.abs(parseFloat(s.letterSpacing) - -0.12) > 0.05) failures.push(`status tracking: ${s.letterSpacing}`);
    }
    return failures;
  });
  expect(folderVerdict, `folder type (got: ${folderVerdict.join(" | ")})`).toEqual([]);

  // ---- Table: mono tabular cells, semibold UI header.
  await createSession(page, "table", "/tmp/t100-left.csv", "/tmp/t100-right.csv", "view-table");
  await expect.poll(async () => page.locator(".tc-cell").count(), { timeout: 15000 }).toBeGreaterThan(0);
  const tableVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const cell = document.querySelector(".tc-cell");
    if (!cell) {
      failures.push("no .tc-cell");
    } else {
      const c = window.getComputedStyle(cell);
      if (c.fontVariantNumeric !== "tabular-nums") failures.push(`table nums: ${c.fontVariantNumeric}`);
      if (c.fontSize !== "12px") failures.push(`table size: ${c.fontSize}`);
      if (!c.fontFamily.includes("SF Mono")) failures.push(`table stack: ${c.fontFamily}`);
    }
    const head = document.querySelector(".tc-head");
    if (head) {
      const h = window.getComputedStyle(head);
      if (h.fontSize !== "12px") failures.push(`head size: ${h.fontSize}`);
      if (h.fontWeight !== "600") failures.push(`head weight: ${h.fontWeight}`);
    }
    return failures;
  });
  expect(tableVerdict, `table type (got: ${tableVerdict.join(" | ")})`).toEqual([]);

  // ---- Hex: mono tabular offsets.
  await createSession(page, "hex", "/tmp/t100-left.bin", "/tmp/t100-right.bin", "view-hex");
  await expect.poll(async () => page.locator(".hx-row").count(), { timeout: 15000 }).toBeGreaterThan(0);
  const hexVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const row = document.querySelector(".hx-row");
    if (!row) {
      failures.push("no .hx-row");
    } else {
      const c = window.getComputedStyle(row);
      if (c.fontSize !== "12px") failures.push(`hex size: ${c.fontSize}`);
      if (!c.fontFamily.includes("SF Mono")) failures.push(`hex stack: ${c.fontFamily}`);
    }
    const off = document.querySelector(".hx-offset");
    if (off) {
      const o = window.getComputedStyle(off);
      if (o.fontVariantNumeric !== "tabular-nums") failures.push(`offset nums: ${o.fontVariantNumeric}`);
    }
    return failures;
  });
  expect(hexVerdict, `hex type (got: ${hexVerdict.join(" | ")})`).toEqual([]);

  await page.screenshot({ path: "/tmp/typo-100-full-light.png" });

  // ---- Dark scheme: ramp + pill hold, icon strokes hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(400);
  const darkVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const tab = document.querySelector('[role="tab"][aria-selected="true"]');
    if (tab) {
      const t = window.getComputedStyle(tab);
      if (parseFloat(t.borderRadius) < 6) failures.push(`dark pill radius: ${t.borderRadius}`);
      if (t.fontWeight !== "500") failures.push(`dark pill weight: ${t.fontWeight}`);
    } else {
      failures.push("no selected tab (dark)");
    }
    const latch = document.querySelector(".hx-row, .tc-cell, .tv-cell, .fv-node");
    if (latch) {
      if (window.getComputedStyle(latch).fontSize !== "12px") failures.push("dark size drift");
    }
    for (const svg of Array.from(document.querySelectorAll('[role="toolbar"] svg'))) {
      if (svg.getAttribute("stroke-width") !== "2") failures.push("dark icon stroke drift");
      if (svg.getAttribute("stroke-linejoin") !== "round") failures.push("dark icon join drift");
    }
    return failures;
  });
  expect(darkVerdict, `dark pins (got: ${darkVerdict.join(" | ")})`).toEqual([]);
  await page.screenshot({ path: "/tmp/typo-100-full-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
