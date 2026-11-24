/**
 * Wave-27 statusbar live diff stats proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows — 623 equal, 4
 * leftOnly, 4 rightOnly, 13 changed) and proves the Beyond Compare
 * signature bottom summary:
 *  - the status bar shows live `+4 -4 ~13 · 644 rows` derived from the
 *    already-loaded ruler rows (no new IPC);
 *  - switching to a folder session clears the counts honestly (no stats
 *    element, bar falls back to kind/tabs + message, layout intact);
 *  - light + dark PNGs land in `/tmp/ui-27/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: every assertion reads the DOM; counts come from the mock
 * IPC fixtures via `App`'s existing ruler rows. Skip policy matches the
 * other specs (infra failures skip).
 */
import { expect, test } from "@playwright/test";

const EVID_DIR = "/tmp/ui-27";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("text session shows live diff counts, folder switch clears honestly", async ({
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

  // Live counts arrive once App pages the full 644-row map into rulerRows.
  // Sequence on the ruler first (same state the counts derive from), then
  // on the stats with a generous timeout for cold dev-server starts.
  await expect(page.getByTestId("ruler")).toHaveAttribute(
    "aria-label",
    "Overview ruler: 644 rows",
    { timeout: 30000 },
  );
  const stats = page.getByTestId("statusbar/stats");
  await expect(stats).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId("statusbar/stats-added")).toHaveText("+4");
  await expect(page.getByTestId("statusbar/stats-removed")).toHaveText("-4");
  await expect(page.getByTestId("statusbar/stats-changed")).toHaveText("~13");
  await expect(page.getByTestId("statusbar/stats-total")).toHaveText("644 rows");
  await expect(stats).toHaveAttribute(
    "aria-label",
    "4 added, 4 removed, 13 changed, 644 total rows",
  );

  // Message side stays intact (honest default, never clobbered by stats).
  await expect(page.getByTestId("statusbar/message")).toBeVisible();
  await expect(page.getByTestId("statusbar")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/statusbar-stats-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/statusbar-stats-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Switching to a folder session clears the text counts honestly: no
  // stats element, bar layout otherwise intact.
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill("/tmp/left-stats");
  await page.getByTestId("session/right-path").fill("/tmp/right-stats");
  await page.getByTestId("session/create-button").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("view-folder")).toBeVisible();
  await expect(page.getByTestId("statusbar/stats")).toHaveCount(0);
  await expect(page.getByTestId("statusbar")).toBeVisible();
  await expect(page.getByTestId("statusbar/message")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
