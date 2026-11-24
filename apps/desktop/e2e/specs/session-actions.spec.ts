/**
 * Wave-25 session row-actions proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `SessionManager` dialog (via the `mountSessionManager`
 * test helper, same as `flows-session.spec.ts`) with two controlled
 * sessions and proves Beyond Compare style row actions:
 *  - rename inline: pencil -> input -> save updates the row name;
 *    empty name renders `session/row-error` inline, never a crash;
 *    cancel restores the original name;
 *  - clone: duplicates the row with a ` (copy)` suffix, original stays;
 *  - favorite: star toggles `aria-pressed` and persists to localStorage
 *    (settings IPC `extra.sessionFavorites` is best-effort under mock);
 *  - delete: first click arms inline Confirm/Cancel; confirm removes the
 *    row when 2+ remain; the last remaining row refuses with
 *    `session/row-error` ("At least one session must remain.");
 *  - dialog still opens/closes cleanly via `onClose`;
 *  - light + dark PNGs land in `/tmp/ui-25/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock-IPC honesty: the mock adapter has NO `session_save` /
 * `settings_set` fixtures (unknown commands resolve `[]`), so
 * persistence is best-effort by design — rows update optimistically and
 * any backend refusal renders inline. Every assertion below reads the
 * DOM or localStorage, never the network. Skip policy matches the other
 * specs (infra failures skip). New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-25";

const SEED = [
  { id: "sess-a", kind: "text", name: "Alpha", locked: false, pageCount: 1 },
  { id: "sess-b", kind: "folder", name: "Beta", locked: false, pageCount: 2 },
];

async function mountManager(page: Page, sessions = SEED) {
  await page.evaluate(async (seed) => {
    const { mountSessionManager } = await import(
      "/src/components/session/index.ts"
    );
    let container = document.getElementById("session-test-container");
    if (!container) {
      container = document.createElement("div");
      container.id = "session-test-container";
      document.body.appendChild(container);
    }
    (window as unknown as Record<string, unknown>).__acClosed = false;
    mountSessionManager(container, {
      isOpen: true,
      sessions: seed,
      activeId: seed[0]?.id ?? null,
      onClose: () => {
        (window as unknown as Record<string, unknown>).__acClosed = true;
      },
    });
  }, sessions);
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("session row actions: rename + clone + favorite + delete guard, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await mountManager(page);
  const dialog = page.getByTestId("session/dialog");
  await expect(dialog).toBeVisible();

  // --- rename: save path ---
  await page.getByTestId("session/rename-sess-a").click();
  const input = page.getByTestId("session/rename-input-sess-a");
  await expect(input).toBeVisible();
  await input.fill("Alpha Renamed");
  await page.getByTestId("session/rename-save-sess-a").click();
  await expect(page.getByTestId("session/item-sess-a")).toContainText("Alpha Renamed");

  // --- rename: empty name is an inline error, never a crash ---
  await page.getByTestId("session/rename-sess-a").click();
  await page.getByTestId("session/rename-input-sess-a").fill("   ");
  await page.getByTestId("session/rename-save-sess-a").click();
  await expect(page.getByTestId("session/row-error")).toContainText("must not be empty");
  // Cancel restores the saved name.
  await page.getByTestId("session/rename-cancel-sess-a").click();
  await expect(page.getByTestId("session/item-sess-a")).toContainText("Alpha Renamed");

  // --- clone: duplicate with copy suffix, original stays ---
  await page.getByTestId("session/clone-sess-a").click();
  const list = page.getByTestId("session/list");
  await expect(list).toContainText("Alpha Renamed (copy)");
  await expect(list).toContainText("Alpha Renamed");
  await expect(list).toContainText("Beta");

  // --- favorite: star toggles and persists to localStorage ---
  const star = page.getByTestId("session/favorite-sess-b");
  await expect(star).toHaveAttribute("aria-pressed", "false");
  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "true");
  const stored = await page.evaluate(() =>
    window.localStorage.getItem("autocompare:sessionFavorites"),
  );
  expect(stored ?? "").toContain("sess-b");
  await expect(page.getByTestId("session/item-sess-b").locator("svg")).not.toHaveCount(0);

  // --- delete: confirm removes a row while 2+ remain ---
  await expect(page.getByTestId("session/item-sess-b")).toBeVisible();
  await page.getByTestId("session/row-delete-sess-b").click();
  await expect(page.getByTestId("session/delete-confirm-sess-b")).toBeVisible();
  await page.getByTestId("session/delete-confirm-sess-b").click();
  await expect(page.getByTestId("session/item-sess-b")).toBeHidden();

  // Light evidence before the last-row guard (clone keeps >= 2 rows).
  await page.screenshot({ path: `${EVID_DIR}/session-actions-light.png` });

  // --- delete: last remaining row refuses inline ---
  // Reduce to a single row by deleting the clone (dynamic id: locate by text).
  const rows = page.getByTestId("session/list").locator('[data-testid^="session/item-"]');
  await expect(rows).toHaveCount(2);
  const copyRow = rows.filter({ hasText: "(copy)" });
  await expect(copyRow).toHaveCount(1);
  await copyRow.getByRole("button", { name: /^Delete / }).click();
  await copyRow.getByRole("button", { name: "Confirm" }).click();
  await expect(rows).toHaveCount(1);
  const lastDelete = page.getByTestId("session/list").getByRole("button", { name: /^Delete / });
  await lastDelete.click();
  await expect(page.getByTestId("session/row-error")).toContainText(
    "At least one session must remain.",
  );
  await expect(page.getByTestId("session/list").locator('[data-testid^="session/item-"]')).toHaveCount(1);

  // --- dialog still closes cleanly ---
  await page.getByRole("button", { name: "Close dialog" }).click();
  const closed = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__acClosed === true,
  );
  expect(closed).toBe(true);

  // --- dark evidence: remount and shoot under dark color scheme ---
  await page.emulateMedia({ colorScheme: "dark" });
  await mountManager(page);
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/session-actions-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
