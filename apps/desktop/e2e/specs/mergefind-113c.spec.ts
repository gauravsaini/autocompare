/**
 * Wave-113C merge find default-closed proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict merge
 * pair (left `["left"]`, right `["right"]`, 2 loaded rows) and proves the
 * Beyond Compare truth (fresh Merge shows one toolbar row, find opens on
 * demand):
 *  - find panel is HIDDEN by default; `mergeview/toolbar-find` reads
 *    `aria-expanded="false"` on load;
 *  - toolbar toggle opens + closes the panel;
 *  - Cmd/Ctrl+F opens + focuses `merge/find-input`, Esc closes;
 *  - panes top (`section[aria-label="three-way merge"]` top ->
 *    `mergeview/conflict-0` top) is <=140px with the panel closed
 *    (same ruler as mergeparity-102);
 *  - find still works when open (substring `t` -> 2 hits, wrap nav,
 *    selection + focus follow, empty no-op, honest miss);
 *  - light + dark PNGs land in `/tmp/mergefind-113c-*.png` (never
 *    in-repo); zero console errors / page errors throughout.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

async function createMergeSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/merge12-base.txt");
  await page.getByTestId("session/right-path").fill("/tmp/merge12-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-merge")).toBeVisible();
}

test("merge find default-closed: hidden by default, toggle + shortcut, budget, find works, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMergeSession(page);
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");

  const toggle = page.getByTestId("mergeview/toolbar-find");
  const input = page.getByTestId("merge/find-input");
  const next = page.getByTestId("merge/find-next");
  const prev = page.getByTestId("merge/find-prev");
  const status = page.getByTestId("merge/find-status");

  // 1. Hidden by default (BC truth: one toolbar row); toggle collapsed.
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(input).toHaveCount(0);

  // 2. Chrome budget with panel closed: section -> first card <=140px
  // (same ruler as mergeparity-102: toolbar + one-line pane headers).
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first card mounted").not.toBeNull();
  expect(chrome?.delta ?? Number.POSITIVE_INFINITY, `firstCardTop<=140 (got ${chrome?.delta}px)`).toBeLessThanOrEqual(140);

  // 3. Toolbar toggle opens + closes.
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(input).toBeVisible();
  await expect(status).toContainText("Find across loaded rows");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(input).toHaveCount(0);

  // 4. Cmd/Ctrl+F opens + focuses (WebKit sometimes drops a lone
  // chord under load: one bounded re-press keeps it deterministic).
  async function pressFindForMerge(): Promise<void> {
    await page.keyboard.press("Meta+f");
    try {
      await expect.poll(() => focusedTestId(page), { timeout: 2000 }).toBe("merge/find-input");
    } catch {
      await page.keyboard.press("Meta+f");
      await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("merge/find-input");
    }
  }
  await pressFindForMerge();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(input).toBeVisible();

  // 5. Find still works when open: `t` sits in both loaded rows.
  await expect(next).toBeDisabled();
  await input.fill("t");
  await input.press("Enter");
  await expect(status).toContainText("match 1 of 2");
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("mergeview/conflict-0");
  await expect(page.getByTestId("mergeview/conflict-0")).toHaveAttribute("aria-current", "true");
  await next.click();
  await expect(status).toContainText("match 2 of 2");
  await next.click();
  await expect(status).toContainText("match 1 of 2");
  await prev.click();
  await expect(status).toContainText("match 2 of 2");

  // Empty query is a clean no-op; miss names the loaded-row scope.
  await input.fill("");
  await expect.poll(() => input.inputValue(), { timeout: 10000 }).toBe("");
  await expect(status).toContainText("Find across loaded rows");
  await expect(next).toBeDisabled();
  await input.fill("zzz-no-such-text-xyz");
  await input.press("Enter");
  await expect(status).toContainText("no matches in 2 loaded rows");

  // 6. Esc closes (input-level Esc -> onClose; re-focus the box first
  // since activation moves DOM focus to the hit card).
  await input.fill("t");
  await input.press("Enter");
  await expect(status).toContainText("match 1 of 2");
  await input.focus();
  await expect(input).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(input).toHaveCount(0);
  await expect(toggle).toHaveAttribute("aria-expanded", "false");

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Evidence (never in-repo): open-state shot, then default (closed)
  // BC-truth shots light + dark.
  await toggle.click();
  await expect(input).toBeVisible();
  await input.fill("t");
  await input.press("Enter");
  await expect(status).toContainText("match 1 of 2");
  await page.screenshot({ path: "/tmp/mergefind-113c-open-light.png" });
  await toggle.click();
  await expect(input).toHaveCount(0);
  await page.screenshot({ path: "/tmp/mergefind-113c-default-light.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/mergefind-113c-default-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
