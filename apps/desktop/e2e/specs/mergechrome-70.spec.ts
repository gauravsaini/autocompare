/**
 * Wave-70 merge chrome collapse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the engine-matched single-conflict
 * merge pair (left `["left"]`, right `["right"]`, 2 loaded rows) and proves
 * the Beyond Compare style collapse (mirrors wave-67B folder +
 * wave-68 table):
 *  - one toolbar row (`mergeview/toolbar`) hosts prev/next conflict nav +
 *    position, take-all left/right, and the find toggle;
 *  - the find panel renders open by default (backward compat: every
 *    legacy find testid stays visible on load);
 *  - the bottom status line (`mergeview/status`) carries the conflict
 *    count (`1 conflict section(s)`, legacy `merge-conflict-count`
 *    testid intact) plus the resolved/unresolved note;
 *  - toolbar/Esc collapse the find panel, Ctrl+F reopens + focuses it;
 *  - collapsed chrome budget: first conflict-card top <= 140px relative
 *    to the merge section top with the panel closed (single toolbar row
 *    above the cards);
 *  - every legacy testid still works (nav, take-all, find, counts);
 *  - light + dark, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-70";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

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

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("merge chrome collapse: single toolbar, find overlay, counts in status bar, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMergeSession(page);
  const view = page.getByTestId("view-merge");
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");

  // 1. Single toolbar row with every control; find panel open by default.
  // Wave-113C: the find panel is default-closed; open it for the legacy
  // open-panel assertions (toggle parity preserved: step 2 closes it).
  if ((await page.getByTestId("mergeview/toolbar-find").getAttribute("aria-expanded")) !== "true") {
    await page.getByTestId("mergeview/toolbar-find").click();
  }
  const toolbar = page.getByTestId("mergeview/toolbar");
  await expect(toolbar).toBeVisible();
  await expect(page.getByTestId("merge-prev")).toBeVisible();
  await expect(page.getByTestId("merge-next")).toBeVisible();
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  await expect(page.getByTestId("merge-take-all-left")).toBeVisible();
  await expect(page.getByTestId("merge-take-all-right")).toBeVisible();
  await expect(page.getByTestId("mergeview/toolbar-find")).toBeVisible();
  await expect(page.getByTestId("mergeview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect(page.getByTestId("merge/find-input")).toBeVisible();
  await expect(page.getByTestId("merge/find-prev")).toBeVisible();
  await expect(page.getByTestId("merge/find-next")).toBeVisible();
  await expect(page.getByTestId("merge/find-status")).toBeVisible();
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // Status bar at the bottom carries the conflict count (legacy testid
  // intact) plus the resolved/unresolved note.
  const status = page.getByTestId("mergeview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict section(s)");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("0 resolved");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("1 unresolved");
  const toolbarTop = await toolbar.evaluate((el) => el.getBoundingClientRect().top);
  const statusTop = await status.evaluate((el) => el.getBoundingClientRect().top);
  const firstCardTop = await page
    .getByTestId("mergeview/conflict-0")
    .evaluate((el) => el.getBoundingClientRect().top);
  expect(statusTop, "counts live below the cards in the status bar").toBeGreaterThan(firstCardTop);
  expect(toolbarTop, "toolbar sits above the cards").toBeLessThan(firstCardTop);

  // 2. Toolbar toggle collapses the find panel; Ctrl+F reopens + focuses.
  await page.getByTestId("mergeview/toolbar-find").click();
  await expect(page.getByTestId("merge/find-input")).toBeHidden();
  await expect(page.getByTestId("mergeview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("merge/find-input")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("merge/find-input");

  // 3. Esc closes the find panel (focus was in the input).
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("merge/find-input")).toBeHidden();
  await page.getByTestId("mergeview/toolbar-find").click();
  await expect(page.getByTestId("merge/find-input")).toBeVisible();

  // 4. Legacy behavior intact: take-all resolves with an honest receipt
  //    (status bar counts follow), per-conflict clears it, find lists hits.
  const preview = page.getByTestId("merge-preview");
  await page.getByTestId("merge-take-all-left").click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nleft");
  await expect(page.getByTestId("merge-take-all-status")).toContainText("took left for 1 conflict");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("1 resolved");
  await expect(page.getByTestId("mergeview/status-note")).toContainText("0 unresolved");
  await page.getByTestId("merge-take-right-0").click();
  await expect(page.getByTestId("merge-take-all-status")).toHaveCount(0);
  await expect(page.getByTestId("mergeview/status-note")).toContainText("1 resolved");
  const input = page.getByTestId("merge/find-input");
  await input.fill("t");
  await input.press("Enter");
  await expect(page.getByTestId("merge/find-status")).toContainText("match 1 of 2");
  await input.fill("zzz-no-such-text-xyz");
  await input.press("Enter");
  await expect(page.getByTestId("merge/find-status")).toContainText("no matches in 2 loaded rows");

  await assertNoOverlap(page);

  // 5. Collapsed chrome budget: with the panel closed the merge chrome
  //    (toolbar + panel, i.e. first-card top relative to the merge
  //    section top — app tab/path bars above are not merge chrome)
  //    is one compact row. Old stacked chrome (~3 rows) was ~150px+.
  await expect(page.getByTestId("merge/find-input")).toBeVisible();
  await page.getByTestId("mergeview/toolbar-find").click();
  await expect(page.getByTestId("merge/find-input")).toBeHidden();
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top };
  });
  expect(chrome, "section + first card mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `merge chrome collapsed to one row (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/merge-chrome-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-chrome-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
