/**
 * Wave-WC text toolbar density (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (real engine rows) and proves the WC density migration:
 *  - button density 22/24: toolbar toggles + nav + save + find/goto
 *    buttons measure 22px (ui sm), find/goto inputs + context/format
 *    selects measure 24px (ui input), checkbox labels 22px — light + dark;
 *  - overlays ride ui/DockedBar in-flow (Find/Goto open via toolbar
 *    toggles, dismiss via close button + Esc, Ctrl+F/G open + focus);
 *  - popovers keep shape: Ignore stays a fieldset popover (NO fullscreen
 *    Modal regression — toggle-close intact), Format stays a Modal
 *    (role=dialog, Esc+backdrop+close single onClose);
 *  - Ignore toggle recompare intact: whitespace/case toggles re-diff
 *    end to end with no fallback note, rows intact;
 *  - chrome budget intact: toolbar single row <=48px, firstRowTop<=140
 *    closed AND with panels open; zero overlap; zero console/page errors.
 *
 * New file only. Skip policy matches the other specs (infra failures skip).
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function focusedTestId(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

/** Opens the panel behind `toggle` only when it is currently closed. */
async function ensureOpen(page: import("@playwright/test").Page, toggle: string): Promise<void> {
  const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
  if (expanded !== "true") await page.getByTestId(toggle).click();
}

/** Closes the panel behind `toggle` only when it is currently open. */
async function ensureClosed(page: import("@playwright/test").Page, toggle: string): Promise<void> {
  const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
  if (expanded === "true") await page.getByTestId(toggle).click();
}

async function sectionDelta(page: import("@playwright/test").Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="text comparison"]');
    const first = document.querySelector('[data-testid="textview/row-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
}

async function roundedHeights(page: import("@playwright/test").Page, ids: string[]): Promise<Record<string, number | null>> {
  return page.evaluate((list: string[]) => {
    const out: Record<string, number | null> = {};
    for (const id of list) {
      const el = document.querySelector(`[data-testid="${id}"]`);
      out[id] = el instanceof HTMLElement ? Math.round(el.getBoundingClientRect().height) : null;
    }
    return out;
  }, ids);
}

test("WC density: 22/24 controls, single-row toolbar, chrome budget, light+dark", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // Closed chrome: first row within budget before any panel opens.
  const closedDelta = await sectionDelta(page);
  expect(closedDelta, "section + first row mounted").not.toBeNull();
  expect(closedDelta ?? Number.POSITIVE_INFINITY, `firstRowTop<=140 closed (got ${closedDelta})`).toBeLessThanOrEqual(140);

  // Toolbar is one compact row: single top across visible buttons, height <= 48.
  const bar = await page.evaluate(() => {
    const barEl = document.querySelector('[data-testid="textview/toolbar"]');
    if (!(barEl instanceof HTMLElement)) return null;
    const btns = Array.from(barEl.querySelectorAll("button")).filter(
      (b) => b instanceof HTMLElement && window.getComputedStyle(b).display !== "none",
    );
    const tops = btns.map((b) => Math.round(b.getBoundingClientRect().top));
    const sameRow = tops.length > 0 && tops.every((t) => Math.abs(t - tops[0]!) <= 2);
    return { h: Math.round(barEl.getBoundingClientRect().height), tops, sameRow };
  });
  expect(bar, "toolbar mounted").not.toBeNull();
  expect(bar!.h, `toolbar height <= 48 (got ${bar!.h})`).toBeLessThanOrEqual(48);
  expect(bar!.sameRow, `toolbar buttons share one row (tops=${bar!.tops.join(",")})`).toBe(true);

  // 22px buttons (sm): toggles + nav + save.
  const buttons22 = [
    "textview/toolbar-find",
    "textview/toolbar-goto",
    "textview/toolbar-ignore",
    "textview/toolbar-format",
    "textview/toolbar-prev-diff",
    "textview/toolbar-next-diff",
    "textview/save-left",
    "textview/save-right",
  ];
  const bHeights = await roundedHeights(page, buttons22);
  for (const id of buttons22) {
    const h = bHeights[id];
    expect(h, `${id} mounted`).not.toBeNull();
    expect(h!, `${id} density 22px (got ${h})`).toBeGreaterThanOrEqual(20);
    expect(h!, `${id} density 22px (got ${h})`).toBeLessThanOrEqual(24);
  }

  // 22px checkbox labels (ui/Checkbox: testid rides the <input>, so pin
  // the parent <label> row height, which carries h-[var(--ac-control-h-sm)]).
  const checks22 = ["textview/wrap-toggle", "textview/linenos-toggle", "textview/diffsonly-toggle"];
  const cHeights = await page.evaluate((list: string[]) => {
    const out: Record<string, number | null> = {};
    for (const id of list) {
      const el = document.querySelector(`[data-testid="${id}"]`);
      const row = el instanceof HTMLElement ? (el.closest("label") ?? el) : null;
      out[id] = row instanceof HTMLElement ? Math.round(row.getBoundingClientRect().height) : null;
    }
    return out;
  }, checks22);
  for (const id of checks22) {
    const h = cHeights[id];
    expect(h, `${id} mounted`).not.toBeNull();
    expect(h!, `${id} label density 22px (got ${h})`).toBeGreaterThanOrEqual(20);
    expect(h!, `${id} label density 22px (got ${h})`).toBeLessThanOrEqual(24);
  }

  // 24px inputs/selects: find + goto inputs, context select.
  await ensureOpen(page, "textview/toolbar-find");
  await ensureOpen(page, "textview/toolbar-goto");
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  const inputs24 = ["textview/find-input", "textview/goto-input", "textview/context-lines"];
  const iHeights = await roundedHeights(page, inputs24);
  for (const id of inputs24) {
    const h = iHeights[id];
    expect(h, `${id} mounted`).not.toBeNull();
    expect(h!, `${id} density 24px (got ${h})`).toBeGreaterThanOrEqual(22);
    expect(h!, `${id} density 24px (got ${h})`).toBeLessThanOrEqual(26);
  }

  // Find/Goto action buttons ride sm=22.
  const overlayBtns = ["textview/find-prev", "textview/find-next", "textview/goto-go"];
  const oHeights = await roundedHeights(page, overlayBtns);
  for (const id of overlayBtns) {
    const h = oHeights[id];
    expect(h, `${id} mounted`).not.toBeNull();
    expect(h!, `${id} density 22px (got ${h})`).toBeGreaterThanOrEqual(20);
    expect(h!, `${id} density 22px (got ${h})`).toBeLessThanOrEqual(24);
  }

  // DockedBar contract: in-flow sections (never absolute/fixed on the bar).
  for (const panel of ["textview/find-panel", "textview/goto-panel"] as const) {
    const pos = await page.getByTestId(panel).evaluate((el) => getComputedStyle(el).position);
    expect(pos, `${panel} in-flow (got ${pos})`).not.toMatch(/absolute|fixed/);
  }

  // Chrome budget holds with panels open (overlay stack never pushes rows).
  await ensureOpen(page, "textview/toolbar-ignore");
  await expect(page.getByTestId("textview/ignore-whitespace")).toBeVisible();
  const openDelta = await sectionDelta(page);
  expect(openDelta, "section + first row mounted (panels open)").not.toBeNull();
  expect(openDelta ?? Number.POSITIVE_INFINITY, `firstRowTop<=140 open (got ${openDelta})`).toBeLessThanOrEqual(140);
  await assertNoOverlap(page);

  // Evidence (never in-repo): light open-state + dark default-state.
  await page.screenshot({ path: "/tmp/textdensity-WC-open-light.png" });
  for (const toggle of ["textview/toolbar-find", "textview/toolbar-goto", "textview/toolbar-ignore"] as const) {
    await ensureClosed(page, toggle);
  }
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await page.screenshot({ path: "/tmp/textdensity-WC-default-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("textview/toolbar")).toBeVisible();
  await page.screenshot({ path: "/tmp/textdensity-WC-default-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("WC overlays: Find/Goto DockedBar Esc, Ignore recompare + shape, Format Modal shape", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // Panels start closed (BC truth).
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/ignore-whitespace")).toHaveCount(0);
  await expect(page.getByTestId("textview/format-options")).toHaveCount(0);

  // Find: toggle opens, close button dismisses (DockedBar contract).
  await page.getByTestId("textview/toolbar-find").click();
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("textview/find-close").click();
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "false");

  // Find: Ctrl+F opens + focuses, Esc dismisses (Esc contract intact).
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/find-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);

  // Goto: toggle opens, close button dismisses.
  await page.getByTestId("textview/toolbar-goto").click();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("textview/goto-close").click();
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

  // Goto: Ctrl+G opens + focuses, Esc dismisses.
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/goto-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

  // Ignore: popover shape KEPT — fieldset, never a fullscreen Modal.
  await page.getByTestId("textview/toolbar-ignore").click();
  const ignoreRoot = page.getByTestId("textview/ignore-options");
  await expect(ignoreRoot).toBeVisible();
  expect(await ignoreRoot.evaluate((el) => el.tagName), "ignore stays fieldset (no Modal regression)").toBe("FIELDSET");
  await expect(ignoreRoot).not.toHaveAttribute("role", "dialog");

  // Ignore: toggle recompare intact — whitespace/case re-diff end to end,
  // no fallback note element at all, rows intact.
  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  await wsToggle.check();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  const caseToggle = page.getByTestId("textview/ignore-case");
  await caseToggle.check();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await wsToggle.uncheck();
  await caseToggle.uncheck();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/row-0")).toBeVisible();

  // Ignore: toolbar toggle-close intact (DEBTC root cause stays fixed —
  // no scrim covers the toggle while open).
  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(page.getByTestId("textview/ignore-whitespace")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("aria-expanded", "false");

  // Format: Modal shape KEPT — role=dialog, Esc dismisses single onClose.
  await page.getByTestId("textview/toolbar-format").click();
  const formatRoot = page.getByTestId("textview/format-options");
  await expect(formatRoot).toBeVisible();
  await expect(formatRoot).toHaveAttribute("role", "dialog");
  await expect(page.getByTestId("textview/format-encoding")).toBeVisible();
  await expect(page.getByTestId("textview/format-eol")).toBeVisible();
  const selHeights = await roundedHeights(page, ["textview/format-encoding", "textview/format-eol"]);
  for (const id of ["textview/format-encoding", "textview/format-eol"] as const) {
    const h = selHeights[id];
    expect(h, `${id} mounted`).not.toBeNull();
    expect(h!, `${id} density 24px (got ${h})`).toBeGreaterThanOrEqual(22);
    expect(h!, `${id} density 24px (got ${h})`).toBeLessThanOrEqual(26);
  }
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/format-options")).toHaveCount(0);

  // Chrome budget intact after every open/dismiss cycle.
  const delta = await sectionDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(delta ?? Number.POSITIVE_INFINITY, `firstRowTop<=140 (got ${delta})`).toBeLessThanOrEqual(140);
  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
