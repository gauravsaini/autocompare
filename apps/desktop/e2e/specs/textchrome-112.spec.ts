/**
 * Wave-112 text chrome de-stack proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-booted demo text diff
 * (real engine rows) and proves the Beyond Compare truth (fresh Text
 * Compare shows one toolbar + status bar, no stacked panels):
 *  - find / goto / ignore panels are HIDDEN by default (wave-112
 *    flips the 66R defaults to closed); toolbar toggles read
 *    `aria-expanded="false"` on load;
 *  - toolbar toggles open + close each panel; Cmd/Ctrl+F opens and
 *    focuses find, Cmd/Ctrl+G opens and focuses goto, Esc closes;
 *  - the disabled Undo control no longer occupies its own row: it is
 *    docked in the bottom status bar (visible + disabled while idle,
 *    below the first row);
 *  - panes top (`section[aria-label="text comparison"]` top ->
 *    `textview/row-0` top) is <= 140px with panels closed (same ruler
 *    as textchrome-89 / textglut-97), i.e. one toolbar row + pathbar;
 *  - light + dark PNGs land in `/tmp/textchrome-112-*.png` (never
 *    in-repo); zero console errors / page errors throughout.
 *
 * New file only. Skip policy matches the other specs (infra failures
 * skip).
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

async function sectionDelta(page: import("@playwright/test").Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="text comparison"]');
    const first = document.querySelector('[data-testid="textview/row-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
}

test("text chrome de-stacked: panels closed by default, toggles + shortcuts, undo docked, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 1. Panels hidden by default; toggles read collapsed.
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);
  await expect(page.getByTestId("textview/ignore-whitespace")).toHaveCount(0);
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("aria-expanded", "false");

  // 2. Disabled Undo occupies no row of its own: docked in the status
  // bar below the list (visible + disabled while idle).
  const undo = page.getByTestId("textview/undo-edit");
  await expect(undo).toBeVisible();
  await expect(undo).toBeDisabled();
  const vertical = await page.evaluate(() => {
    const u = document.querySelector('[data-testid="textview/undo-edit"]');
    const first = document.querySelector('[data-testid="textview/row-0"]');
    const toolbar = document.querySelector('[data-testid="textview/toolbar"]');
    if (!(u instanceof HTMLElement) || !(first instanceof HTMLElement) || !(toolbar instanceof HTMLElement))
      return null;
    return {
      undoTop: u.getBoundingClientRect().top,
      firstTop: first.getBoundingClientRect().top,
      toolbarBottom: toolbar.getBoundingClientRect().bottom,
    };
  });
  expect(vertical, "undo + first row + toolbar mounted").not.toBeNull();
  expect(vertical!.undoTop, "undo docked below the list (status bar)").toBeGreaterThan(vertical!.firstTop);
  expect(vertical!.toolbarBottom, "toolbar sits above the list").toBeLessThanOrEqual(vertical!.firstTop);

  // 3. Row budget with panels closed: section -> first-row <= 140px.
  const delta = await sectionDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(delta ?? Number.POSITIVE_INFINITY, `firstRowTop<=140 (got ${delta}px)`).toBeLessThanOrEqual(140);

  // 4. Toolbar toggles open + close each panel.
  await page.getByTestId("textview/toolbar-find").click();
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-find")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("textview/toolbar-find").click();
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);

  await page.getByTestId("textview/toolbar-goto").click();
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-goto")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("textview/toolbar-goto").click();
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(page.getByTestId("textview/ignore-whitespace")).toBeVisible();
  await expect(page.getByTestId("textview/toolbar-ignore")).toHaveAttribute("aria-expanded", "true");
  await page.getByTestId("textview/toolbar-ignore").click();
  await expect(page.getByTestId("textview/ignore-whitespace")).toHaveCount(0);

  // 5. Shortcuts: Cmd/Ctrl+F opens + focuses find, Esc closes;
  // Cmd/Ctrl+G opens + focuses goto, Esc closes.
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/find-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);

  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("textview/goto-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("textview/goto-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/goto-input")).toHaveCount(0);

  // 6. Collapsed chrome still drives the legacy controls (wrap smoke)
  // and the layout stays overlap-free.
  const wrapToggle = page.getByTestId("textview/wrap-toggle");
  await wrapToggle.check();
  await expect(page.locator('section[aria-label="text comparison"]')).toHaveAttribute("data-wrap", "on");
  await wrapToggle.uncheck();
  await expect(page.locator('section[aria-label="text comparison"]')).toHaveAttribute("data-wrap", "off");
  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Evidence (never in-repo): fresh default-state shots + panels-open.
  await ensureOpen(page, "textview/toolbar-find");
  await ensureOpen(page, "textview/toolbar-goto");
  await ensureOpen(page, "textview/toolbar-ignore");
  await page.screenshot({ path: "/tmp/textchrome-112-panels-open-light.png" });
  // Default (closed) state is the BC truth shot: close all again.
  for (const toggle of ["textview/toolbar-find", "textview/toolbar-goto", "textview/toolbar-ignore"] as const) {
    const expanded = await page.getByTestId(toggle).getAttribute("aria-expanded");
    if (expanded === "true") await page.getByTestId(toggle).click();
  }
  await expect(page.getByTestId("textview/find-input")).toHaveCount(0);
  await page.screenshot({ path: "/tmp/textchrome-112-default-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/textchrome-112-default-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
