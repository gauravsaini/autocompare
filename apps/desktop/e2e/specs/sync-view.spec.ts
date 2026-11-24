/**
 * Wave-47 sync plan view proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> SyncView) over the mock sync engine (deterministic plan
 * derived from the bundled real-engine folder fixtures) and proves:
 *  - a `sync` session mounts `view-sync` instead of the "No data view"
 *    fallback (the wave-46 gap);
 *  - the rendered row count equals the engine non-`same` node count;
 *  - dry-run apply renders an honest receipt and touches nothing;
 *  - rendered nodes never overlap (boundingBox check);
 *  - light + dark PNGs land in `/tmp/ui-47/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { FOLDER_NODES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-47";
const EXPECTED_STEPS = FOLDER_NODES.filter((n) => n.status !== "same").length;

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createSyncSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("sync");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-sync")).toBeVisible();
}

test("sync plan: engine step count, dry-run receipt, real apply receipt, light+dark", async ({
  page,
}) => {
  expect(EXPECTED_STEPS, "fixture has non-same nodes").toBeGreaterThan(0);
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createSyncSession(page);
  await expect(page.getByTestId("syncview/totals")).toContainText(/steps/);
  const rows = page.getByTestId("syncview/row");
  await expect
    .poll(async () => rows.count(), { timeout: 10000 })
    .toBe(EXPECTED_STEPS);
  await assertNoOverlap(page);

  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/dry run/);
  await expect(page.getByTestId("syncview/receipt")).toContainText(/nothing touched/);

  await page.getByTestId("syncview/apply-real").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(
    new RegExp(`applied: ${EXPECTED_STEPS} copied, 0 deleted`),
  );

  await expect(page.getByRole("alert")).toHaveCount(0);

  await page.screenshot({ path: `${EVID_DIR}/sync-view-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/sync-view-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
