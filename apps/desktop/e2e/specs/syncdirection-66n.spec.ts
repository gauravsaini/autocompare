/**
 * Wave-66N sync direction + prune proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> SyncView) over the mock sync engine and proves:
 *  - the direction selector boots left-wins (`syncview/dir-left`
 *    `aria-pressed="true"`, others `"false"`) with prune unchecked and
 *    NO opts on the wire (`window.__SYNC_OPTS_LAST__ === null` — the
 *    default wire stays byte-identical to the pre-66N `sync_plan`
 *    call);
 *  - switching direction flips `aria-pressed` and re-plans (steps
 *    reload) with explicit opts `{direction:"NewerWins",prune:false}`;
 *  - enabling prune re-plans with `prune:true`;
 *  - the mode note (`syncview/mode-note`) echoes the effective mode;
 *  - direction/prune changes reset the apply receipt but keep the name
 *    filter text;
 *  - dry-run + real apply receipts still work after a direction change
 *    (composition smoke);
 *  - light + dark PNGs land in `/tmp/ui-66n/` (never in-repo);
 *  - `assertNoOverlap` holds and zero console/page errors occur.
 *
 * Mock-IPC honesty: the mock `sync_plan` ignores direction (fixed
 * left-wins plan; `src/ipc/client.ts` is wave-frozen), so the request
 * path is proven the same way `table-bykey.spec.ts` proves its IPC
 * param — via the headless-proof hook the view mirrors
 * (`__SYNC_OPTS_LAST__`) — plus the full plan -> apply loop, never a
 * crash. The REAL direction-honoring path is covered by the engine
 * `sync_plan` Rust unit tests. Skip policy matches the other specs
 * (infra failures skip). New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { FOLDER_NODES } from "../helpers/real-fixtures";
import { assertNoOverlap } from "../helpers/overlap";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-66n";
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

async function lastOpts(page: Page): Promise<unknown> {
  return page.evaluate(
    () => (window as unknown as Record<string, unknown>).__SYNC_OPTS_LAST__,
  );
}

test("sync direction+prune: defaults, re-plan request path, mode note, apply smoke, light+dark", async ({
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

  const dirLeft = page.getByTestId("syncview/dir-left");
  const dirRight = page.getByTestId("syncview/dir-right");
  const dirNewer = page.getByTestId("syncview/dir-newer");
  const pruneToggle = page.getByTestId("syncview/prune-toggle");
  const modeNote = page.getByTestId("syncview/mode-note");
  const rows = page.getByTestId("syncview/row");

  // Boot defaults: left-wins pressed, prune off, no opts on the wire,
  // full fixed left-wins plan rendered.
  await expect(dirLeft).toBeVisible();
  await expect(dirLeft).toHaveAttribute("aria-pressed", "true");
  await expect(dirRight).toHaveAttribute("aria-pressed", "false");
  await expect(dirNewer).toHaveAttribute("aria-pressed", "false");
  await expect(dirLeft).toContainText("Update right (left wins)");
  await expect(dirRight).toContainText("Update left (right wins)");
  await expect(dirNewer).toContainText("Newer wins");
  await expect(pruneToggle).not.toBeChecked();
  await expect.poll(() => lastOpts(page)).toBeNull();
  await expect(modeNote).toBeVisible();
  await expect(modeNote).toContainText("direction: left to right");
  await expect(modeNote).toContainText("prune: off");
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);

  // Seed a receipt + a name filter, then switch direction: receipt
  // resets, filter text is kept, explicit opts go on the wire.
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/dry run/);

  const paths = await page.$$eval('[data-testid="syncview/row"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const query = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  const filter = page.getByTestId("syncview/name-filter");
  await filter.fill(query);
  await expect(page.getByTestId("syncview/name-note")).toBeVisible();

  await dirNewer.click();
  await expect(dirNewer).toHaveAttribute("aria-pressed", "true");
  await expect(dirLeft).toHaveAttribute("aria-pressed", "false");
  await expect(dirRight).toHaveAttribute("aria-pressed", "false");
  await expect
    .poll(() => lastOpts(page))
    .toEqual({ direction: "NewerWins", prune: false });
  await expect(modeNote).toContainText("direction: newer wins");
  await expect(modeNote).toContainText("prune: off");
  // Re-plan: receipt reset, filter text kept, steps reloaded.
  await expect(page.getByTestId("syncview/receipt")).toHaveCount(0);
  await expect(filter).toHaveValue(query);
  await expect(page.getByTestId("syncview/name-note")).toBeVisible();
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // Enabling prune re-plans with prune:true; mode note follows.
  await pruneToggle.check();
  await expect
    .poll(() => lastOpts(page))
    .toEqual({ direction: "NewerWins", prune: true });
  await expect(modeNote).toContainText("direction: newer wins");
  await expect(modeNote).toContainText("prune: on");
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Composition smoke: dry-run + real apply receipts still work after
  // the direction change (mock still serves the fixed plan).
  await filter.fill("");
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(EXPECTED_STEPS);
  await page.getByTestId("syncview/apply-dry").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(/dry run/);
  await expect(page.getByTestId("syncview/receipt")).toContainText(/nothing touched/);
  await page.getByTestId("syncview/apply-real").click();
  await expect(page.getByTestId("syncview/receipt")).toContainText(
    new RegExp(`applied: ${EXPECTED_STEPS} copied, 0 deleted`),
  );
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Direction back to left-wins with prune still on keeps explicit opts.
  await dirLeft.click();
  await expect(dirLeft).toHaveAttribute("aria-pressed", "true");
  await expect
    .poll(() => lastOpts(page))
    .toEqual({ direction: "LeftToRight", prune: true });
  await expect(modeNote).toContainText("direction: left to right");
  await expect(modeNote).toContainText("prune: on");

  // Light + dark evidence (never in-repo).
  await assertNoOverlap(page, 'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]');
  await page.screenshot({ path: `${EVID_DIR}/syncdirection-66n-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/syncdirection-66n-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
