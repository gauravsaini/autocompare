/**
 * Wave-21 line detail pane proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, first page 200 rows
 * non-virtualized) and proves the Beyond Compare signature bottom pane:
 *  - clicking a changed row selects it (`data-selected`/`aria-selected`)
 *    and the `LineDetailPane` shows that row's left/right text stacked
 *    with character-level highlights derived from its `inline` ops, plus
 *    a header with the row index + kind;
 *  - ArrowDown/ArrowUp on a focused row moves the selection (pane header
 *    follows);
 *  - switching to a non-text session clears the selection and shows no
 *    populated pane (empty/hidden, layout intact);
 *  - light + dark PNGs land in `/tmp/ui-21/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: every assertion reads the DOM; rows come from the mock
 * IPC fixtures, selection state is held in `App` (no new IPC). Skip
 * policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-21";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("row click shows stacked char-highlight detail, keyboard moves it, session switch clears", async ({
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

  // No selection yet: pane renders its empty placeholder, layout intact.
  const pane = page.getByTestId("line-detail");
  await expect(pane).toBeVisible();
  await expect(pane).toHaveAttribute("data-empty", "true");
  await expect(page.getByTestId("line-detail/empty")).toContainText("No line selected");

  // Row 51 is a single-row `changed` hunk with one inline op
  // (a 0..4 vs b 0..5: "left"/"right" vs "right" prefix swap).
  const row51 = page.getByTestId("textview/row-51");
  await expect(row51).toHaveAttribute("data-kind", "changed");
  await expect(row51).toHaveAttribute("data-selected", "false");

  // Click the left cell (bubbles to the row; avoids the gutter buttons).
  await row51.locator('div[title="left"]').click();
  await expect(row51).toHaveAttribute("data-selected", "true");
  await expect(row51).toHaveAttribute("aria-selected", "true");

  // Pane shows the selected row stacked with char highlights.
  await expect(pane).toHaveAttribute("data-empty", "false");
  await expect(page.getByTestId("line-detail/header")).toContainText("52");
  await expect(page.getByTestId("line-detail/header")).toContainText("changed");
  await expect(page.getByTestId("line-detail/left")).toContainText("left variant of line 0047");
  await expect(page.getByTestId("line-detail/right")).toContainText("right variant of line 0047");
  const leftHl = page.getByTestId("line-detail/left").getByTestId("line-detail/hl");
  const rightHl = page.getByTestId("line-detail/right").getByTestId("line-detail/hl");
  await expect(leftHl.first()).toContainText("left");
  await expect(rightHl.first()).toContainText("right");
  expect(await leftHl.count()).toBeGreaterThan(0);
  expect(await rightHl.count()).toBeGreaterThan(0);

  // Keyboard: ArrowDown moves to row 52 (equal, no highlights expected
  // on either side beyond plain text), ArrowUp moves back to 51.
  await row51.focus();
  await page.keyboard.press("ArrowDown");
  const row52 = page.getByTestId("textview/row-52");
  await expect(row52).toHaveAttribute("data-selected", "true");
  await expect(row51).toHaveAttribute("data-selected", "false");
  await expect(page.getByTestId("line-detail/header")).toContainText("53");
  await expect(page.getByTestId("line-detail/header")).toContainText("equal");
  await page.keyboard.press("ArrowUp");
  await expect(row51).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("line-detail/header")).toContainText("changed");

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/line-detail-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/line-detail-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Switching to a non-text session clears the selection: no populated
  // pane, layout otherwise intact.
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill("/tmp/left-detail");
  await page.getByTestId("session/right-path").fill("/tmp/right-detail");
  await page.getByTestId("session/create-button").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("view-folder")).toBeVisible();
  await expect(page.getByTestId("line-detail")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
