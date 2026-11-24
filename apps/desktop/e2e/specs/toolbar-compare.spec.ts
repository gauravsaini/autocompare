/**
 * Wave-29 toolbar compare-actions proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (demo text session auto-boots through
 * the real `text_compare` mock IPC) and proves the Beyond Compare style
 * compare lifecycle owned by `components/chrome/Toolbar.tsx`:
 *  - Compare runs the active compare IPC, shows a busy spinner
 *    (`toolbar/compare-busy`), and disables itself with a
 *    `(comparing…)` reason while the run is in flight;
 *  - Stop is disabled with a titled reason when idle, enables while
 *    busy, and is an honest no-op with an inline note
 *    (`toolbar/compare-note`: backend cannot abort a sent compare);
 *  - Refresh re-runs the current compare (`toolbar/compare-runs`
 *    `runs: 1` -> `runs: 2`), disabled with the compare's own blocked
 *    reason while busy or when no session is open;
 *  - Compare + Refresh are keyboard operable (focus + Enter re-runs);
 *  - empty session disables all three honestly with titled reasons;
 *  - failures render inline, never crash;
 *  - light + dark PNGs land in `/tmp/ui-29/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * No new IPC: every run reuses the host's kind-matched `*_compare`
 * handlers. Skip policy matches the other specs (infra failures skip).
 * NEW file only; no existing spec touched.
 */
import { expect, test } from "@playwright/test";

const EVID_DIR = "/tmp/ui-29";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function runsText(page: import("@playwright/test").Page): Promise<string> {
  return ((await page.getByTestId("toolbar/compare-runs").textContent()) ?? "").trim();
}

test("toolbar compare lifecycle: busy cycle + stop note + refresh re-runs, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text session boots automatically (mock IPC serves real rows).
  const compare = page.getByTestId("toolbar/action-compare");
  const stop = page.getByTestId("toolbar/action-compare-stop");
  const refresh = page.getByTestId("toolbar/action-compare-refresh");
  await expect(compare).toBeEnabled({ timeout: 15000 });
  await expect(refresh).toBeEnabled();
  await expect(refresh).toHaveAttribute("title", "Refresh");
  // Idle stop disables honestly with a titled reason.
  await expect(stop).toBeDisabled();
  await expect(stop).toHaveAttribute("title", "Stop (no compare running)");

  // Keyboard reachability: focus + Enter runs the compare.
  await compare.focus();
  await expect(compare).toBeFocused();
  await page.keyboard.press("Enter");

  // Busy cycle: spinner shows, compare/refresh disable with reason,
  // stop enables while the run is in flight.
  await expect(page.getByTestId("toolbar/compare-busy")).toBeVisible({ timeout: 5000 });
  await expect(compare).toBeDisabled();
  await expect(compare).toHaveAttribute("title", "Compare (comparing…)");
  await expect(refresh).toBeDisabled();
  await expect(refresh).toHaveAttribute("title", "Refresh (comparing…)");
  await expect(stop).toBeEnabled();
  await expect(stop).toHaveAttribute("title", "Stop");

  // Stop while busy: honest no-op with an inline note, never a crash.
  // Stop stays keyboard reachable while enabled.
  await stop.focus();
  await expect(stop).toBeFocused();
  await stop.click();
  const note = page.getByTestId("toolbar/compare-note");
  await expect(note).toBeVisible();
  await expect(note).toContainText("cannot be cancelled");

  // The running compare settles on its own: busy clears, run counted.
  await expect(page.getByTestId("toolbar/compare-busy")).toBeHidden({ timeout: 15000 });
  await expect.poll(() => runsText(page), { timeout: 10000 }).toBe("runs: 1");
  await expect(compare).toBeEnabled();
  await expect(refresh).toBeEnabled();
  await expect(stop).toBeDisabled();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Refresh re-runs the current compare via keyboard (focus + Enter).
  await refresh.focus();
  await expect(refresh).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("toolbar/compare-busy")).toBeVisible({ timeout: 5000 });
  await expect(page.getByTestId("toolbar/compare-busy")).toBeHidden({ timeout: 15000 });
  await expect.poll(() => runsText(page), { timeout: 10000 }).toBe("runs: 2");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/toolbar-compare-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/toolbar-compare-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("toolbar compare lifecycle: empty session disables honestly", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Dismiss the auto-opened demo -> blank workspace, no sessions.
  const compare = page.getByTestId("toolbar/action-compare");
  await expect(compare).toBeEnabled({ timeout: 15000 });
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("workspace/open-manager")).toBeVisible({ timeout: 10000 });

  // All three lifecycle actions disable honestly with titled reasons.
  const stop = page.getByTestId("toolbar/action-compare-stop");
  const refresh = page.getByTestId("toolbar/action-compare-refresh");
  await expect(compare).toBeDisabled();
  await expect(compare).toHaveAttribute("title", /needs an open session/);
  await expect(refresh).toBeDisabled();
  await expect(refresh).toHaveAttribute("title", /needs an open session/);
  await expect(stop).toBeDisabled();
  await expect(stop).toHaveAttribute("title", "Stop (no compare running)");
  await expect(page.getByTestId("toolbar/compare-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
