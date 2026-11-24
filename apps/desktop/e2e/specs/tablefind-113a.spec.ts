/**
 * Wave-113A table find default-closed proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics; header `id | name | price`, 550
 * rows, 100 loaded) and proves the Beyond Compare parity gap is closed
 * (Table Compare showed an always-open Find panel while BC shows one
 * toolbar):
 *  - the find panel renders CLOSED by default (`table-find-input`
 *    count 0, `tableview/toolbar-find` `aria-expanded="false"`);
 *  - with the panel closed the first grid row sits within the
 *    toolbar+pathbar budget (section top -> first row <= 140px, same
 *    ruler as tablegrid-91 / folderchrome-98);
 *  - the toolbar Find toggle opens the panel (input visible,
 *    `aria-expanded="true"`) and find still works from there
 *    (`10.` -> `Hit 1 of 6`);
 *  - Esc collapses the panel, Ctrl+F reopens + focuses
 *    `table-find-input`;
 *  - before/after PNGs land in `/tmp/tablefind-113a-*.png`
 *    (never in-repo); light + dark, zero console/page errors.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

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
  await page.getByTestId("session/left-path").fill("/tmp/wave20-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave20-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("tablefind-113A: find closed by default, toggle + Ctrl+F open it, chrome budget holds", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("table-cols-count")).toContainText("100 / 550 rows");

  // 1. Find panel closed by default (BC parity: one toolbar, no
  //    always-open panel).
  await expect(page.getByTestId("tableview/toolbar-find")).toBeVisible();
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await expect(page.getByTestId("table-find-input")).toHaveCount(0);
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);

  // 2. Collapsed chrome budget: section top -> first grid row <= 140px
  //    with the find panel closed (same ruler as tablegrid-91 /
  //    folderchrome-98).
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector('[data-testid="tableview/grid-left"] tbody tr.tc-row');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first grid row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `closed-panel chrome within budget (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  await page.screenshot({ path: "/tmp/tablefind-113a-closed-light.png" });

  // 3. Toolbar toggle opens the panel; find still works from there.
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  const input = page.getByTestId("table-find-input");
  await expect(input).toBeVisible();
  await input.fill("10.");
  await page.getByTestId("table-find-search").click();
  await expect(page.getByTestId("table-find-status")).toContainText("Hit 1 of 6", {
    timeout: 10000,
  });
  await expect(page.locator('[data-hit="true"]')).toHaveCount(3);
  await page.screenshot({ path: "/tmp/tablefind-113a-open-light.png" });

  // 4. Esc collapses the panel; Ctrl+F reopens + focuses the input.
  await expect(input).toBeVisible();
  await input.focus();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-find-input")).toHaveCount(0);
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await page.keyboard.press("Control+f");
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await expect(page.getByTestId("tableview/toolbar-find")).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("table-find-input");

  // 5. Dark scheme: closed default + open panel both render clean.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-find-input")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("tableview/status")).toBeVisible();
  await page.screenshot({ path: "/tmp/tablefind-113a-closed-dark.png" });
  await page.getByTestId("tableview/toolbar-find").click();
  await expect(page.getByTestId("table-find-input")).toBeVisible();
  await page.screenshot({ path: "/tmp/tablefind-113a-open-dark.png" });

  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
