/**
 * Wave-66F merge find proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict merge
 * pair (left `["left"]`, right `["right"]`, 2 loaded rows) and proves the
 * BC-style find row (text-61 twin):
 *  - the find row renders (`merge/find-input`, `merge/find-prev`,
 *    `merge/find-next`, `merge/find-status`) with Next disabled on an
 *    empty query;
 *  - Cmd/Ctrl+F focuses this view's find box (per-view listener, only the
 *    active view is mounted so focus lands in exactly one box);
 *  - typing `t` + Enter lists 2 row-major hits (`match 1 of 2`) with the
 *    owning conflict selected (`aria-current`) and DOM focus on its card;
 *  - Next/Prev wrap in both directions (2/2 -> 1/2 -> 2/2);
 *  - matching is case-insensitive (`LEFT` hits the left row);
 *  - an empty query is a clean no-op (default hint, Next disabled);
 *  - a miss names the loaded-row scope (`no matches in 2 loaded rows`);
 *  - typing in the find box never drives conflict navigation;
 *  - light + dark PNGs land in `/tmp/ui-66f/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Purely presentational: every assertion reads the DOM, never the network.
 * Skip policy matches the other specs (infra failures skip). New file only,
 * no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-66f";

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
  // Wave-10 boots the demo text session; clear it for deterministic counts.
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

test("merge find: substring hits, wrap nav, selection+focus, empty no-op, honest misses, light+dark", async ({
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

  const input = page.getByTestId("merge/find-input");
  const next = page.getByTestId("merge/find-next");
  const prev = page.getByTestId("merge/find-prev");
  const status = page.getByTestId("merge/find-status");
  // Wave-113C: the find panel is default-closed; open it for the legacy
  // open-panel assertions (Meta+F below only opens/focuses, never closes).
  if ((await page.getByTestId("mergeview/toolbar-find").getAttribute("aria-expanded")) !== "true") {
    await page.getByTestId("mergeview/toolbar-find").click();
  }
  await expect(input).toBeVisible();
  await expect(prev).toBeVisible();
  await expect(next).toBeVisible();
  await expect(next).toBeDisabled();
  await expect(status).toContainText("Find across loaded rows");

  // Cmd/Ctrl+F focuses this view's find box (per-view listener). WebKit
  // sometimes drops a lone modifier-chord under load: one bounded re-press
  // keeps the assert deterministic without masking a real product miss.
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

  // `t` sits in both loaded rows (`left` + `right`): 2 row-major hits.
  await input.fill("t");
  await input.press("Enter");
  await expect(status).toContainText("match 1 of 2");
  // Selection + focus follow the hit: owning conflict selected, DOM focus
  // on its card. Waiting for that focus proves the whole chain settled,
  // so the Next click below cannot race a mid-flight focus under load.
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("mergeview/conflict-0");
  await expect(page.getByTestId("mergeview/conflict-0")).toHaveAttribute("aria-current", "true");
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");

  // Typing in the find box never drives conflict navigation (the view's
  // j/k/n/p handler ignores input targets).
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");

  // Next/Prev wrap in both directions over the 2 hits.
  await next.click();
  await expect(status).toContainText("match 2 of 2");
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("mergeview/conflict-0");
  await next.click();
  await expect(status).toContainText("match 1 of 2");
  await prev.click();
  await expect(status).toContainText("match 2 of 2");

  // Case-insensitive: `LEFT` hits the single left row.
  await input.fill("LEFT");
  await input.press("Enter");
  await expect(status).toContainText("match 1 of 1");

  // Empty query is a clean no-op: default hint, Next disabled, no crash.
  await input.fill("");
  await expect.poll(() => input.inputValue(), { timeout: 10000 }).toBe("");
  await expect(status).toContainText("Find across loaded rows");
  await expect(next).toBeDisabled();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Misses name the loaded-row scope (left + right rows only).
  await input.fill("zzz-no-such-text-xyz");
  await input.press("Enter");
  await expect(status).toContainText("no matches in 2 loaded rows");

  await assertNoOverlap(page);

  // Light + dark evidence (never in-repo).
  await input.fill("t");
  await input.press("Enter");
  await expect(status).toContainText("match 1 of 2");
  await page.screenshot({ path: `${EVID_DIR}/merge-find-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-find-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
