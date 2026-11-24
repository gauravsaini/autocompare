/**
 * Wave-87 table two-pane layout proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics; header `id | name | price`, 550
 * rows, 100 loaded) and proves the Beyond Compare style two-pane
 * header (mirrors the TextView pane grid):
 *  - a `tableview/panes` grid renders left/right panes side by side
 *    (`tableview/pane-left` fully left of `tableview/pane-right`,
 *    thin `tv-splitter` center rail between the `1fr` columns);
 *  - each pane owns its path line (`tableview/path-left` /
 *    `tableview/path-right` naming the session inputs);
 *  - the mock carries no file meta, so `tableview/meta`,
 *    `tableview/meta-left`, `tableview/meta-right` render NOTHING
 *    (honest-hidden fallback, no crash, no fabricated values);
 *  - the bottom status bar (`tableview/status`) still shows the
 *    FULL-table total from the summary (`tableview/diffs` with
 *    `125 difference row(s)`) below the grid;
 *  - collapsed chrome budget holds with both panels closed: first
 *    grid row within 140px of the table section top;
 *  - light + dark PNGs land in `/tmp/ui-87/` (never in-repo).
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-87";

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

async function createTableSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave87-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave87-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("table two-pane: side-by-side panes, per-side paths, diff-count status, honest-hidden mock meta, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  const view = page.getByTestId("view-table");
  await expect(page.getByTestId("table-cols-count")).toContainText("100 / 550 rows");

  // 1. Two panes visible side by side with the splitter between them.
  const panes = page.getByTestId("tableview/panes");
  const paneLeft = page.getByTestId("tableview/pane-left");
  const paneRight = page.getByTestId("tableview/pane-right");
  await expect(panes).toBeVisible();
  await expect(paneLeft).toBeVisible();
  await expect(paneRight).toBeVisible();
  const sideBySide = await page.evaluate(() => {
    const l = document.querySelector('[data-testid="tableview/pane-left"]');
    const r = document.querySelector('[data-testid="tableview/pane-right"]');
    if (!(l instanceof HTMLElement) || !(r instanceof HTMLElement)) return null;
    const lb = l.getBoundingClientRect();
    const rb = r.getBoundingClientRect();
    return { left: lb.x, leftRight: lb.x + lb.width, right: rb.x, rightBottom: rb.y + rb.height };
  });
  expect(sideBySide, "both panes mounted").not.toBeNull();
  expect(sideBySide?.left ?? 0, "left pane starts left of right pane").toBeLessThan(
    sideBySide?.right ?? 0,
  );
  expect(
    sideBySide?.leftRight ?? Number.POSITIVE_INFINITY,
    `left pane ends where right begins (got ${sideBySide?.leftRight} vs ${sideBySide?.right})`,
  ).toBeLessThanOrEqual((sideBySide?.right ?? 0) + 8);

  // 2. Each pane owns its path line naming the session inputs.
  await expect(page.getByTestId("tableview/path-left")).toContainText("wave87-left.csv");
  await expect(page.getByTestId("tableview/path-right")).toContainText("wave87-right.csv");

  // 3. Honest-hidden mock fallback: the mock carries no file meta, so
  //    nothing extra renders — no crash, no fabricated values.
  await expect(page.getByTestId("tableview/meta")).toHaveCount(0);
  await expect(page.getByTestId("tableview/meta-left")).toHaveCount(0);
  await expect(page.getByTestId("tableview/meta-right")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // 4. Bottom diff-count status bar keeps the FULL-table total below
  //    the grid (mock fixture: 125 diff rows over 550 total rows).
  const status = page.getByTestId("tableview/status");
  await expect(status).toBeVisible();
  await expect(page.getByTestId("tableview/diffs")).toContainText("125 difference row(s)");
  const stacked = await page.evaluate(() => {
    const first = document.querySelector("tbody tr.tc-row");
    const bar = document.querySelector('[data-testid="tableview/status"]');
    if (!(first instanceof HTMLElement) || !(bar instanceof HTMLElement)) return null;
    return {
      firstTop: first.getBoundingClientRect().top,
      barTop: bar.getBoundingClientRect().top,
    };
  });
  expect(stacked, "grid + status bar mounted").not.toBeNull();
  expect(
    stacked?.barTop ?? 0,
    "status bar lives below the grid",
  ).toBeGreaterThan(stacked?.firstTop ?? 0);

  // 5. Collapsed chrome budget: with both panels closed the first row
  //    still sits within 140px of the table section top (the pane
  //    header adds one compact row, not stacked chrome).
  // Wave-113A note: find panel is already default-closed (no toggle needed).
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `two-pane chrome stays collapsed (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/table-ours-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/table-ours-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
