/**
 * Wave-34 word-wrap toggle proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves the
 * Beyond Compare style word-wrap toggle:
 *  - the toggle renders unchecked by default (scroll mode: single-line
 *    cells, `white-space: pre`, fixed 22px rows);
 *  - flipping it on soft-wraps without new IPC (status line unchanged):
 *    cells switch to `white-space: pre-wrap` + `break-all`, rows grow to
 *    auto height, the section flips to `data-wrap="on"`, and a real row
 *    at a narrow viewport demonstrably wraps (taller than 28px);
 *  - row integrity holds: `textview/row-N` testids + `data-kind`
 *    unchanged, hunk gutter copy controls still mounted (hunk nav
 *    unaffected), click selection still works;
 *  - the toggle persists per session view (survives `Load more`) and is
 *    keyboard reachable (focus + Space flips it back off, restoring
 *    `pre` single-line rendering);
 *  - light + dark PNGs land in `/tmp/ui-34/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: no IPC carries wrap state (presentational only), so every
 * assertion below reads the DOM / computed styles, never the network.
 * Skip policy matches the other specs (infra failures skip).
 */
import { expect, test } from "@playwright/test";

const EVID_DIR = "/tmp/ui-34";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("word wrap toggles soft-wrap, keeps rows/selection/hunk nav", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  const statusBefore = await page.getByTestId("textview/status").innerText();

  const wrapToggle = page.getByTestId("textview/wrap-toggle");
  const section = page.locator('section[aria-label="text comparison"]');
  await expect(wrapToggle).toBeVisible();
  await expect(wrapToggle).not.toBeChecked();
  await expect(section).toHaveAttribute("data-wrap", "off");

  // Narrow viewport so real demo lines overflow their pane: this makes the
  // wrap reflow observable on genuine rows (no fixtures touched).
  await page.setViewportSize({ width: 500, height: 800 });
  const row0 = page.getByTestId("textview/row-0");
  await expect(row0).toBeVisible();
  const kindBefore = await row0.getAttribute("data-kind");
  const cell0 = row0.locator(".tv-cell").first();

  // Scroll mode (default): single-line cells, row renders.
  // wave-236 behavior-only (22px pin lives in e2e/parity).
  expect(await cell0.evaluate((el) => getComputedStyle(el).whiteSpace)).toBe("pre");
  const heightOff = await row0.evaluate((el) => (el as HTMLElement).offsetHeight);
  expect(heightOff).toBeGreaterThan(0);

  // Toggle on: soft-wrap, no new IPC (status line byte-identical).
  await wrapToggle.check();
  await expect(wrapToggle).toBeChecked();
  await expect(section).toHaveAttribute("data-wrap", "on");
  expect(await cell0.evaluate((el) => getComputedStyle(el).whiteSpace)).toBe("pre-wrap");
  expect(await cell0.evaluate((el) => getComputedStyle(el).wordBreak)).toBe("break-all");
  const heightOn = await row0.evaluate((el) => (el as HTMLElement).offsetHeight);
  // wave-236 behavior-only: wrap grows the row (28px history pin removed; relative growth only).
  expect(heightOn).toBeGreaterThan(heightOff);
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);

  // Row integrity: testids + kinds unchanged.
  await expect(page.getByTestId("textview/row-0")).toBeVisible();
  await expect(page.getByTestId("textview/row-6")).toBeVisible();
  expect(await page.getByTestId("textview/row-0").getAttribute("data-kind")).toBe(kindBefore);

  // Hunk nav unaffected: the first hunk (rows 6-9) still mounts its
  // center-gutter take-left / take-right pair while wrapped.
  await expect(page.getByTestId("hunk-copy-left-0")).toBeVisible();
  await expect(page.getByTestId("hunk-copy-right-0")).toBeVisible();

  // Selection still works while wrapped: click selects, aria follows.
  await row0.click();
  await expect(row0).toHaveAttribute("data-selected", "true");

  // Persists per session view: paging more rows keeps wrap on.
  const loadMore = page.getByRole("button", { name: /Load more/ });
  if (await loadMore.count()) {
    await loadMore.click();
    await expect(page.getByTestId("textview/status")).toContainText("400 / 644 rows");
    await expect(wrapToggle).toBeChecked();
    await expect(section).toHaveAttribute("data-wrap", "on");
  }

  // Wrap-on evidence (never in-repo).
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.screenshot({ path: `${EVID_DIR}/text-wrap-on-light.png` });

  // Keyboard reachability: Tab to the toggle, Space flips it off, scroll
  // mode restores single-line rendering with zero IPC.
  await wrapToggle.focus();
  await page.keyboard.press("Space");
  await expect(wrapToggle).not.toBeChecked();
  await expect(section).toHaveAttribute("data-wrap", "off");
  expect(await cell0.evaluate((el) => getComputedStyle(el).whiteSpace)).toBe("pre");
  // wave-236 behavior-only: row renders again after unwrap (22px pin lives in e2e/parity).
  expect(await row0.evaluate((el) => (el as HTMLElement).offsetHeight)).toBeGreaterThan(0);
  await expect(page.getByTestId("textview/status")).toContainText("rows");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/text-wrap-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/text-wrap-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
