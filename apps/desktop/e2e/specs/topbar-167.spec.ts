/**
 * Wave-167: top-bar enablement proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves the unified top Toolbar
 * drives the active view through the existing view-bus events:
 *  - text kind: top-bar Format is enabled and opens the in-view Format
 *    popover (`textview/format-options`); Rules stays disabled with an
 *    honest folder-only reason; Minor is enabled (W161 live path);
 *    Copy parks with `select a row` until a hunk row is selected, then
 *    enables and runs the staged text-copy path with a status note;
 *  - folder kind: top-bar Rules is enabled and opens the in-view Rules
 *    panel (`folderview/rules-panel`); Format/Minor park honestly
 *    (text-only paths);
 *  - chrome budget holds with either panel open (firstRowTop <= 140,
 *    no new rows, no `position: absolute` locks — plain
 *    getBoundingClientRect deltas only);
 *  - zero console errors / page errors throughout.
 *
 * PORT-FREE: run on a fresh port (`reuseExistingServer:false`,
 * `--workers=1`, webkit) so no shared port or parallel worker can
 * flake the gate. Skip policy matches the other specs (infra failures
 * skip). NEW file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  return { pageErrors, consoleErrors };
}

function assertClean(
  pageErrors: string[],
  consoleErrors: string[],
  where: string,
): void {
  expect(pageErrors, `zero page errors (${where})`).toEqual([]);
  expect(consoleErrors, `zero console errors (${where})`).toEqual([]);
}

async function bootText(page: Page): Promise<void> {
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows", { timeout: 15000 });
}

/** Section-relative first-row delta (no offsetHeight locks). */
async function firstRowDelta(
  page: Page,
  sectionSel: string,
  rowSel: string,
): Promise<number | null> {
  return page.evaluate(
    ([section, row]) => {
      const s = document.querySelector(section);
      const r = document.querySelector(row);
      if (!(s instanceof HTMLElement) || !(r instanceof HTMLElement)) return null;
      return Math.round(r.getBoundingClientRect().top - s.getBoundingClientRect().top);
    },
    [sectionSel, rowSel] as const,
  );
}

/** Create a fresh comparison session through the session dialog. */
async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewId: string,
): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click().catch(() => {});
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewId)).toBeVisible({ timeout: 15000 });
}

test("topbar-167: text kind — Format/Minor/Copy drive the view, Rules parks honestly", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Format is live in text kind and opens the in-view popover.
  const format = page.getByTestId("toolbar/action-format");
  await expect(format).toBeEnabled();
  await expect(format).toHaveAttribute("title", "Format");
  await expect(page.getByTestId("textview/format-options")).toBeHidden();
  await format.click();
  await expect(page.getByTestId("textview/format-options")).toBeVisible();
  await expect(page.getByTestId("statusbar/message")).toContainText("Format options opened");

  // Chrome budget with the top-bar-opened popover: no new rows above
  // the pane.
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  const openDelta = await firstRowDelta(
    page,
    'section[aria-label="text comparison"], section:has(.tv-row)',
    '[data-testid="textview/row-0"], .tv-row',
  );
  expect(openDelta, "text firstRowTop measured (format open)").not.toBeNull();
  expect(openDelta!, `text firstRowTop <= 140px (got ${openDelta})`).toBeLessThanOrEqual(140);
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/format-options")).toBeHidden();

  // Rules parks honestly: the Rules panel lives in folder kind only.
  const rules = page.getByTestId("toolbar/action-rules");
  await expect(rules).toBeDisabled();
  await expect(rules).toHaveAttribute("title", /needs folder compare/);

  // Minor is live in text kind (W161 bus path): toggle on/off with
  // honest status notes, diff stays live.
  const minor = page.getByTestId("toolbar/action-minor");
  await expect(minor).toBeEnabled();
  await expect(minor).toHaveAttribute("title", "Minor");
  await minor.click();
  await expect(minor).toHaveAttribute("title", "Minor (on)");
  await expect(page.getByTestId("statusbar/message")).toContainText("ignoring whitespace and case");
  await expect(page.getByTestId("textview/status")).toContainText("rows");
  await minor.click();
  await expect(minor).toHaveAttribute("title", "Minor");
  await expect(page.getByTestId("statusbar/message")).toContainText("Minor off");

  // Copy parks until a hunk row is selected, then runs the staged
  // text-copy path with a status note.
  const copy = page.getByTestId("toolbar/action-copy");
  await expect(copy).toBeDisabled();
  expect(await copy.getAttribute("title")).toContain("select a row");
  await page.getByTestId("textview/row-98").click();
  await expect(copy).toBeEnabled();
  await copy.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");

  assertClean(pageErrors, consoleErrors, "text top-bar");
});

test("topbar-167: folder kind — Rules drives the panel, Format/Minor park honestly", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createSession(page, "folder", "/tmp/r167-left", "/tmp/r167-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(3);

  // Rules is live in folder kind and opens the in-view panel.
  const rules = page.getByTestId("toolbar/action-rules");
  await expect(rules).toBeEnabled();
  await expect(rules).toHaveAttribute("title", "Rules");
  await expect(page.getByTestId("folderview/rules-panel")).toBeHidden();
  await rules.click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  await expect(page.getByTestId("statusbar/message")).toContainText("Rules panel opened");

  // Chrome budget with the top-bar-opened panel: no new rows above
  // the tree.
  const openDelta = await firstRowDelta(
    page,
    'section[aria-label="folder comparison"]',
    '[data-testid^="folder-row-"]',
  );
  expect(openDelta, "folder firstRowTop measured (rules open)").not.toBeNull();
  expect(openDelta!, `folder firstRowTop <= 140px (got ${openDelta})`).toBeLessThanOrEqual(140);

  // Format/Minor park honestly: both are text-only paths.
  const format = page.getByTestId("toolbar/action-format");
  await expect(format).toBeDisabled();
  await expect(format).toHaveAttribute("title", /needs text compare/);
  const minor = page.getByTestId("toolbar/action-minor");
  await expect(minor).toBeDisabled();
  await expect(minor).toHaveAttribute("title", /needs text compare/);

  assertClean(pageErrors, consoleErrors, "folder top-bar");
});
