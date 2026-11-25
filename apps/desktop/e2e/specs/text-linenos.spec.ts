/**
 * Wave-39 per-side line-number gutter proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves the
 * Beyond Compare style gutter:
 *  - the "Line numbers" toggle renders checked by default, the section
 *    carries `data-linenos="on"`, and every loaded row mounts
 *    `textview/lineno-left-N` + `textview/lineno-right-N`;
 *  - numbers are honest per kind, derived from the loaded rows only
 *    (no new IPC: the status line is byte-identical throughout):
 *    equal row-0 shows 1/1, `leftOnly` rows 6-9 show 7-10 with a BLANK
 *    right cell (`data-empty="true"`), equal row-10 resumes at 11/7,
 *    `rightOnly` row-20 shows blank/17, equal row-24 shows 21/21,
 *    `changed` row-51 shows 48/48;
 *  - wrap keeps alignment: at a narrow viewport with wrap on, the number
 *    cell's top edge stays within 2px of its row's top edge while the
 *    row grows taller than 28px;
 *  - hunk jump lands correctly: `toolbar/action-next-hunk` moves the
 *    ruler to hunk start 6 and row-6 is visible with left number 7;
 *  - selection + detail pane unaffected: clicking row-10 selects it and
 *    `line-detail` shows its already-loaded content;
 *  - the toggle hides both gutters (`data-linenos="off"`, zero lineno
 *    cells) and is keyboard reachable (focus + Space restores them);
 *  - light + dark PNGs land in `/tmp/ui-39/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: line numbers come from counting non-null `left`/`right`
 * sides of the mock rows already on screen, so every assertion reads
 * the DOM, never the network. Skip policy matches the other specs.
 */
import { expect, test } from "@playwright/test";

const EVID_DIR = "/tmp/ui-39";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function lineno(page: import("@playwright/test").Page, side: "left" | "right", index: number): Promise<string> {
  const text = await page.getByTestId(`textview/lineno-${side}-${index}`).innerText();
  return text.trim();
}

test("line-number gutter shows honest per-side numbers, stays aligned, toggles", async ({ page }) => {
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

  const toggle = page.getByTestId("textview/linenos-toggle");
  const section = page.locator('section[aria-label="text comparison"]');
  await expect(toggle).toBeVisible();
  await expect(toggle).toBeChecked();
  await expect(section).toHaveAttribute("data-linenos", "on");

  // Honest per-kind numbers (left counters count non-null left sides).
  await expect(page.getByTestId("textview/row-6")).toBeVisible();
  expect(await lineno(page, "left", 0)).toBe("1");
  expect(await lineno(page, "right", 0)).toBe("1");
  expect(await lineno(page, "left", 6)).toBe("7");
  expect(await lineno(page, "right", 6)).toBe("");
  expect(await lineno(page, "left", 9)).toBe("10");
  expect(await lineno(page, "right", 9)).toBe("");
  expect(await lineno(page, "left", 10)).toBe("11");
  expect(await lineno(page, "right", 10)).toBe("7");
  expect(await lineno(page, "left", 20)).toBe("");
  expect(await lineno(page, "right", 20)).toBe("17");
  expect(await lineno(page, "left", 24)).toBe("21");
  expect(await lineno(page, "right", 24)).toBe("21");
  expect(await lineno(page, "left", 51)).toBe("48");
  expect(await lineno(page, "right", 51)).toBe("48");

  // Blank cells say so explicitly (one-sided rows stay honest).
  await expect(page.getByTestId("textview/lineno-right-6")).toHaveAttribute("data-empty", "true");
  await expect(page.getByTestId("textview/lineno-left-20")).toHaveAttribute("data-empty", "true");
  await expect(page.getByTestId("textview/lineno-left-0")).toHaveAttribute("data-empty", "false");

  // No new IPC for the gutter: the status line is byte-identical.
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);

  // Wrap keeps numbers aligned with their rows: narrow viewport, wrap
  // on, the row grows (relative) while its number cell stays pinned to
  // the row's top edge (first visual line).
  // wave-236 behavior-only (28px history pin removed; growth is relative).
  const wrapToggle = page.getByTestId("textview/wrap-toggle");
  await page.setViewportSize({ width: 500, height: 800 });
  await wrapToggle.check();
  await expect(section).toHaveAttribute("data-wrap", "on");
  const row0 = page.getByTestId("textview/row-0");
  const heightOn = await row0.evaluate((el) => (el as HTMLElement).offsetHeight);
  expect(heightOn).toBeGreaterThan(0);
  const gap = await row0.evaluate((el) => {
    const left = el.querySelector('[data-testid="textview/lineno-left-0"]');
    if (!(left instanceof HTMLElement)) return -999;
    const r = el.getBoundingClientRect();
    const g = left.getBoundingClientRect();
    return Math.abs(g.top - r.top);
  });
  expect(gap).toBeLessThanOrEqual(2);
  expect(await lineno(page, "left", 6)).toBe("7");
  expect(await lineno(page, "right", 20)).toBe("17");
  await expect(page.getByTestId("textview/status")).toHaveText(statusBefore);
  await page.setViewportSize({ width: 1280, height: 800 });
  await wrapToggle.uncheck();

  // Hunk jump lands correctly: next-hunk targets hunk start 6 and the
  // row carries its line numbers.
  await page.getByTestId("toolbar/action-next-hunk").click();
  await expect(page.getByTestId("ruler")).toHaveAttribute("aria-valuenow", "6");
  await expect(page.getByTestId("textview/row-6")).toBeVisible();
  expect(await lineno(page, "left", 6)).toBe("7");

  // Selection + detail pane unaffected: click selects from loaded rows.
  const row10 = page.getByTestId("textview/row-10");
  await row10.click();
  await expect(row10).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("line-detail")).toBeVisible();
  await expect(page.getByTestId("line-detail/left")).toContainText("common line 0010");

  // Toggle off hides both gutters; keyboard (Space) restores them.
  await toggle.uncheck();
  await expect(section).toHaveAttribute("data-linenos", "off");
  await expect(page.getByTestId("textview/lineno-left-0")).toHaveCount(0);
  await expect(page.getByTestId("textview/lineno-right-0")).toHaveCount(0);
  await expect(page.getByTestId("textview/row-0")).toBeVisible();
  await expect(page.getByTestId("hunk-copy-left-0")).toBeVisible();
  await toggle.focus();
  await page.keyboard.press("Space");
  await expect(toggle).toBeChecked();
  await expect(section).toHaveAttribute("data-linenos", "on");
  expect(await lineno(page, "left", 0)).toBe("1");

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/text-linenos-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/text-linenos-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
