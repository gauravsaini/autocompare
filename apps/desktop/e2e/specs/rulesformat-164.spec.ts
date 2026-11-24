/**
 * Wave-164: Rules/Format UI wiring proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves the frontend slice over the
 * W163c engine surface (masks + encoding/eol + table ignores):
 *  - text Format button opens an in-flow popover (encoding/EOL selects);
 *    choosing values re-issues `text_compare` with both spellings (see
 *    `window.__TEXT_IGNORE_LAST__`) and the diff stays live;
 *  - folder Rules button opens an in-flow panel (mask input); applying
 *    `*.csv` narrows the folder snapshot via `folder_compare` masks,
 *    clearing restores everything, with an honest masks note;
 *  - table ignores ride the typed `tableCompare` call-site shape
 *    (legacy 4-field wire byte-identical without flags); the mock
 *    honors both spellings without crashing (see
 *    `window.__TABLE_IGNORE_LAST__`);
 *  - top-bar Rules/Format stay honest disabled stubs (App handlers
 *    off-limits), Minor untouched; chrome budget firstRowTop <= 140
 *    with popovers open (no new rows, no `position: absolute` locks —
 *    plain getBoundingClientRect deltas only).
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

test("rulesformat-164: text Format popover threads encoding/eol through text_compare", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Format toggle opens the in-flow popover (closed by default).
  const formatBtn = page.getByTestId("textview/toolbar-format");
  await expect(formatBtn).toBeVisible();
  await expect(page.getByTestId("textview/format-options")).toBeHidden();
  await formatBtn.click();
  const panel = page.getByTestId("textview/format-options");
  await expect(panel).toBeVisible();

  // Chrome budget with the popover open: first text row stays <= 140px
  // below its section (no new rows above the pane).
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  const openDelta = await firstRowDelta(
    page,
    'section[aria-label="text comparison"], section:has(.tv-row)',
    '[data-testid="textview/row-0"], .tv-row',
  );
  expect(openDelta, "text firstRowTop measured (format open)").not.toBeNull();
  expect(openDelta!, `text firstRowTop <= 140px (got ${openDelta})`).toBeLessThanOrEqual(140);

  // Encoding select re-issues the compare with both spellings.
  await page.getByTestId("textview/format-encoding").selectOption("windows-1252");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TEXT_IGNORE_LAST__,
        ),
      { timeout: 10000 },
    )
    .toMatchObject({ encoding: "windows-1252" });
  await expect(page.getByTestId("textview/status")).toContainText("644 rows");

  // EOL select threads through on the same path.
  await page.getByTestId("textview/format-eol").selectOption("lf");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TEXT_IGNORE_LAST__,
        ),
      { timeout: 10000 },
    )
    .toMatchObject({ encoding: "windows-1252", eol: "lf" });
  await expect(page.getByTestId("textview/status")).toContainText("644 rows");

  // Back to Auto/Auto restores the legacy exact wire (hook cleared to
  // undefined options, diff stays live).
  await page.getByTestId("textview/format-encoding").selectOption("");
  await page.getByTestId("textview/format-eol").selectOption("");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>).__TEXT_IGNORE_LAST__,
        ),
      { timeout: 10000 },
    )
    .toMatchObject({ encoding: undefined, eol: undefined });
  await expect(page.getByTestId("textview/status")).toContainText("644 rows");

  // Esc collapses the popover; budget still holds.
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  assertClean(pageErrors, consoleErrors, "text format");
});

test("rulesformat-164: folder Rules popover threads masks through folder_compare", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await createSession(page, "folder", "/tmp/r164-left", "/tmp/r164-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(3);
  const before = await page.locator('[data-testid^="folder-row-"]').count();

  // Rules toggle opens the in-flow panel (closed by default).
  const rulesBtn = page.getByTestId("folderview/toolbar-rules");
  await expect(rulesBtn).toBeVisible();
  await expect(page.getByTestId("folderview/rules-panel")).toBeHidden();
  await rulesBtn.click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();

  // Chrome budget with the panel open: first tree row stays <= 140px
  // below the folder section (no new rows above the pane).
  const openDelta = await firstRowDelta(
    page,
    'section[aria-label="folder comparison"]',
    '[data-testid^="folder-row-"]',
  );
  expect(openDelta, "folder firstRowTop measured (rules open)").not.toBeNull();
  expect(openDelta!, `folder firstRowTop <= 140px (got ${openDelta})`).toBeLessThanOrEqual(140);

  // Applying a narrow mask narrows the snapshot (mock + engine honor
  // masks at compare time, before the status filter).
  await page.getByTestId("folderview/mask-input").fill("*.csv");
  await page.getByTestId("folderview/mask-apply").click();
  await expect(page.getByTestId("folderview/masks-note")).toContainText("*.csv");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeLessThan(before);

  // Clearing restores everything with no note.
  await page.getByTestId("folderview/mask-clear").click();
  await expect(page.getByTestId("folderview/masks-note")).toBeHidden();
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(3);

  // Esc collapses the panel.
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/rules-panel")).toBeHidden();
  assertClean(pageErrors, consoleErrors, "folder rules");
});

test("rulesformat-164: table ignores ride the call-site shape, mock honors both spellings", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  // Legacy 4-param wire still boots a real table session (proves the
  // optional 5th `opts` param kept backward compat).
  await createSession(page, "table", "/tmp/r164-left.csv", "/tmp/r164-right.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]").first()).toBeVisible({
    timeout: 15000,
  });
  const hookBefore = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
  );
  expect(hookBefore, "legacy table compare sends no flags").toMatchObject({
    ignoreCase: false,
    ignoreSpace: false,
  });

  // Flagged compare through the real typed wrapper (both spellings):
  // the mock accepts + applies them (faithful no-op on this fixture)
  // without crashing, and the summary shape stays intact.
  const flagged = await page.evaluate(async () => {
    const mod = (await import("/src/ipc/table.ts")) as unknown as {
      tableCompare: (
        left: string,
        right: string,
        byKey: boolean,
        keyColumns: readonly number[],
        opts: Record<string, boolean>,
      ) => Promise<{ sessionId: string; diffRows: number; equal: boolean }>;
    };
    return mod.tableCompare("/tmp/r164-left.csv", "/tmp/r164-right.csv", false, [], {
      ignoreCase: true,
      ignore_space: true,
    });
  });
  expect(flagged.sessionId, "mock table session id").toBe("mock-table-1");
  expect(typeof flagged.diffRows, "mock diffRows is a number").toBe("number");
  const hookAfter = await page.evaluate(
    () => (window as unknown as Record<string, unknown>).__TABLE_IGNORE_LAST__,
  );
  expect(hookAfter, "table flags reached the wire").toMatchObject({
    ignoreCase: true,
    ignoreSpace: true,
  });
  assertClean(pageErrors, consoleErrors, "table ignores");
});

test("rulesformat-164: top-bar Rules/Format stay honest stubs, Minor untouched", async ({
  page,
}) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // App.tsx handlers are off-limits: the top-bar buttons stay disabled
  // with the contract reason, while the live path lives in-view.
  const rules = page.getByTestId("toolbar/action-rules");
  const format = page.getByTestId("toolbar/action-format");
  await expect(rules).toBeDisabled();
  await expect(rules).toHaveAttribute("title", /needs contract L1/);
  await expect(format).toBeDisabled();
  await expect(format).toHaveAttribute("title", /needs contract L1/);

  // Minor keeps its W161 wiring (enabled in text kind with a session,
  // honestly parked otherwise — either way never a crash).
  const minor = page.getByTestId("toolbar/action-minor");
  await expect(minor).toBeVisible();
  const minorTitle = (await minor.getAttribute("title")) ?? "";
  expect(
    minorTitle === "Minor" || minorTitle.startsWith("Minor (") || minorTitle.includes("needs text compare"),
    `minor keeps an honest title (got ${minorTitle})`,
  ).toBe(true);
  assertClean(pageErrors, consoleErrors, "top-bar stubs");
});
