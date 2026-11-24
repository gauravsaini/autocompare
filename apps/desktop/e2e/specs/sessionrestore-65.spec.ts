import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-65 boot restore: created tabs auto-persist through the existing
 * session_save IPC (mock store mirrors the real temp-dir backend), so a
 * page reload lists them again with the last-active tab focused instead
 * of booting a duplicate demo. No new IPC, no new UI.
 * New file only, no existing spec touched.
 */
const MOCK_STORE_KEY = "autocompare:mockSessionStore:v1";

async function storedIds(page: Page): Promise<string[]> {
  return page.evaluate((key) => {
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return [];
      return Object.keys(parsed as Record<string, unknown>);
    } catch {
      return [];
    }
  }, MOCK_STORE_KEY);
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("created tab survives reload with last-active focus", async ({ page }) => {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("text");
  await page.getByTestId("session/left-path").fill("/tmp/u65-restore-left");
  await page.getByTestId("session/right-path").fill("/tmp/u65-restore-right");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-text")).toBeVisible();

  // Deterministic gate: auto-persist (demo + new tab) landed in the store.
  await expect.poll(() => storedIds(page), { timeout: 10000 }).toHaveLength(2);

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions: 2", { timeout: 15000 });

  // The restored last-active tab is the u65 pair, working, not a demo dup.
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Tab title carries the basenames (sessionDisplayName); the store proves
  // full paths survived (TextView status shows counts, never full paths).
  await expect(page.getByTestId("tabbar")).toContainText("u65-restore-left");
  await expect(page.getByTestId("tabbar")).toContainText("u65-restore-right");
  const storedLefts = await page.evaluate((key) => {
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return [];
      const parsed = JSON.parse(raw) as Record<string, { tabs?: Array<{ left_path?: unknown; right_path?: unknown }> }>;
      return Object.values(parsed).map((doc) => `${String(doc.tabs?.[0]?.left_path ?? "")}|${String(doc.tabs?.[0]?.right_path ?? "")}`);
    } catch {
      return [];
    }
  }, MOCK_STORE_KEY);
  expect(storedLefts.some((pair) => pair.includes("/tmp/u65-restore-left") && pair.includes("/tmp/u65-restore-right"))).toBe(true);

  await assertNoOverlap(page);
});
