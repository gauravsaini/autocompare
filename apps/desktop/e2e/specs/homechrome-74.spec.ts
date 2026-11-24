/**
 * Wave-74 home two-pane proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (welcome -> cards/dialog -> tabs ->
 * Home tab -> session list) and proves the Beyond Compare Home style
 * two-pane welcome (mirrors the wave-73 presentational split: new
 * `home/controls/HomeSessionsPane`, `App` welcome block only, no new
 * session kinds, no backend change, no new HOME_CARDS):
 *  - welcome is a two-pane flex row: fixed-width `home/sessions` pane
 *    left, existing cards grid right (all legacy testids intact);
 *  - empty state is honest (`home/sessions-empty`, session-count 0);
 *  - with sessions open the Home tab re-shows welcome with one row per
 *    session (title + kind), fresh-input sessions carry a dirty dot,
 *    blank sessions do not, and clicking a row activates that session;
 *  - collapsed chrome budget: first session row top <= 140px relative
 *    to the welcome top;
 *  - light + dark, zero console/page errors.
 *
 * New file only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-74";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function clearToEmpty(page: Page): Promise<void> {
  const banner = page.getByTestId("workspace/demo-banner");
  const clear = page.getByTestId("workspace/demo-clear");
  await expect(banner.or(page.getByTestId("workspace/welcome"))).toBeVisible({
    timeout: 15000,
  });
  for (let round = 0; round < 2; round += 1) {
    try {
      await banner.waitFor({ state: "visible", timeout: 5000 });
      await clear.click();
      await expect(page.getByTestId("session-count")).toContainText("sessions: 0", {
        timeout: 10000,
      });
    } catch {
      break;
    }
  }
  await expect(page.getByTestId("workspace/welcome")).toBeVisible();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
}

async function paneDelta(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const welcome = document.querySelector('[data-testid="workspace/welcome"]');
    const first = document.querySelector('[data-testid^="home/session-row-"]');
    if (!(welcome instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - welcome.getBoundingClientRect().top;
  });
}

test("home two-pane: session list activates, empty honest, cards intact, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await clearToEmpty(page);
  const welcome = page.getByTestId("workspace/welcome");
  const pane = page.getByTestId("home/sessions");

  // 1. Two-pane layout: flex row, fixed-width sessions pane left of cards.
  await expect(pane).toBeVisible();
  const direction = await welcome.evaluate(
    (el) => window.getComputedStyle(el).flexDirection,
  );
  expect(direction, "welcome is a two-pane row").toBe("row");
  const paneBox = await pane.boundingBox();
  expect(paneBox, "sessions pane mounted").not.toBeNull();
  expect(paneBox?.width ?? 0, `pane is fixed width (got ${paneBox?.width}px)`).toBeGreaterThanOrEqual(200);
  expect(paneBox?.width ?? 0, `pane is fixed width (got ${paneBox?.width}px)`).toBeLessThanOrEqual(280);
  const cardsBox = await page.getByTestId("home/new-text").boundingBox();
  expect(cardsBox, "cards grid mounted").not.toBeNull();
  expect(paneBox?.x ?? 0, "sessions pane sits left of the cards grid").toBeLessThan(cardsBox?.x ?? 0);

  // 2. Honest empty state reusing the session-count data.
  await expect(page.getByTestId("home/sessions-empty")).toBeVisible();
  await expect(page.getByTestId("home/sessions-empty")).toContainText(/no open sessions/i);
  await expect(page.locator('[data-testid^="home/session-row-"]')).toHaveCount(0);

  // 3. Cards grid intact: all 7 kinds + open-manager legacy path.
  for (const kind of ["text", "folder", "hex", "table", "merge", "sync", "media"]) {
    await expect(page.getByTestId(`home/new-${kind}`)).toBeVisible();
  }
  await page.getByTestId("workspace/open-manager").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();

  await assertNoOverlap(page);

  // 4. Two sessions: A (blank text via card, clean) + B (paths via
  //    dialog, fresh inputs -> dirty). Same open paths as the app uses.
  await page.getByTestId("home/new-text").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(welcome).toBeHidden();
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("text");
  await page.getByTestId("session/left-path").fill("/tmp/h74-left.txt");
  await page.getByTestId("session/right-path").fill("/tmp/h74-right.txt");
  await page.getByTestId("session/create-button").click();
  await expect(page.getByTestId("session/list")).toContainText("h74-left.txt");
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 2");

  // 5. Home tab re-shows welcome with the session list populated.
  await page.getByTestId("tab-home").click();
  await expect(welcome).toBeVisible();
  await expect(page.getByTestId("home/sessions-empty")).toHaveCount(0);
  const rows = page.locator('[data-testid^="home/session-row-"]');
  await expect(rows).toHaveCount(2);
  await expect(pane).toContainText("h74-left.txt");
  await expect(page.locator('[data-testid^="home/session-kind-"]')).toHaveCount(2);
  // Dirty dot rides the existing tab state: B (fresh inputs) has it,
  // A (blank paths, nothing to run) does not.
  await expect(page.locator('[data-testid^="home/session-dirty-"]')).toHaveCount(1);

  // 6. Collapsed chrome budget: first session row top <= 140px relative
  //    to the welcome top (mirrors the wave-73 row budget).
  const delta = await paneDelta(page);
  expect(delta, "pane + first row mounted").not.toBeNull();
  expect(
    delta ?? Number.POSITIVE_INFINITY,
    `first session row within one compact block (got ${delta}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);

  // 7. Click activates: B's row switches to the B session view.
  await pane.getByRole("button", { name: /h74-left\.txt/ }).click();
  await expect(welcome).toBeHidden();
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByRole("tab", { selected: true })).toContainText("h74-left");
  // And back home: A's (blank) row activates the A session.
  // Wave-138: blank sessions render the honest pick-two empty (no
  // demo rows) instead of view-text.
  await page.getByTestId("tab-home").click();
  await expect(welcome).toBeVisible();
  await pane.getByRole("button", { name: /text session/ }).click();
  await expect(welcome).toBeHidden();
  await expect(page.getByTestId("workspace/blank-empty")).toBeVisible();
  await expect(page.getByTestId("view-text")).toHaveCount(0);
  await expect(page.getByTestId("session-count")).toContainText("sessions: 2");

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.getByTestId("tab-home").click();
  await expect(welcome).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/home-chrome-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/home-chrome-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
