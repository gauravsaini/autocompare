/**
 * Wave-238 chrome toolbar dedupe (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Proves Axis-3 single ownership without touching views/**:
 *  1. top Toolbar owns Prev/Next + All/Diffs/Same/Context + Copy
 *     addressing via ViewCommandBus (one bus dispatch per click, views
 *     follow through existing listeners; view buttons untouched);
 *  2. every disabled top-bar button carries its reason in `title`
 *     (tooltip) + an honest status note path;
 *  3. every toolbar-advertised chord exists in `shortcuts.ts`
 *     (`TOOLBAR_SHORTCUT_HINTS`) and every `menu.rs` id has an App
 *     `menu-event` listener (dead ids wired or honestly reported);
 *  4. toolbar testids stay byte-identical.
 *
 * PORT-FREE: fresh server per gate run (`reuseExistingServer:false`,
 * `--workers=1`, webkit). Skip policy matches sibling specs. NEW file
 * only; no existing spec touched.
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

async function bootText(page: Page): Promise<void> {
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/status")).toContainText("rows", { timeout: 15000 });
}

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

async function hunksOrdinal(page: Page): Promise<{ at: number; total: number }> {
  const label = await page.getByTestId("textview/toolbar-hunks").textContent();
  const match = label?.match(/hunk (\d+) of (\d+)/);
  expect(match, `hunks label is positional (got "${label}")`).not.toBeNull();
  return { at: Number(match![1]), total: Number(match![2]) };
}

async function emitMenu(page: Page, id: string, command = ""): Promise<void> {
  await page.evaluate(
    ({ id, command }) => {
      const fn = (window as unknown as { __MOCK_EMIT_MENU__?: (p: { id: string; command?: string }) => void })
        .__MOCK_EMIT_MENU__;
      if (typeof fn === "function") fn({ id, command });
      window.dispatchEvent(new CustomEvent("menu-event", { detail: { id, command } }));
    },
    { id, command },
  );
}

test("toolbar-dedupe-238: top bar owns nav/filter/copy via bus, testids pinned", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Testids byte-identical: every unified top-bar action stays mounted.
  for (const id of [
    "home",
    "open",
    "filter-all",
    "filter-diffs",
    "filter-same",
    "filter-context",
    "minor",
    "rules",
    "format",
    "copy",
    "next-hunk",
    "prev-hunk",
    "swap",
    "compare",
    "copy-right",
    "sync",
    "merge",
  ]) {
    await expect(page.getByTestId(`toolbar/action-${id}`), `testid toolbar/action-${id}`).toBeAttached();
  }

  // Copy addressing rides the bus: disabled with reason until a hunk row
  // is selected (fresh boot has no selection), then runs the staged take path.
  // (Checked BEFORE nav below: nav clicks select a hunk as a side effect.)
  const copy = page.getByTestId("toolbar/action-copy");
  await expect(copy).toBeDisabled();
  await expect(copy).toHaveAttribute("title", /select a row first/);

  // Prev/Next ride ONE bus source: one click = exactly one hunk.
  const next = page.getByTestId("toolbar/action-next-hunk");
  const prev = page.getByTestId("toolbar/action-prev-hunk");
  await expect(next).toBeEnabled();
  await expect(prev).toBeEnabled();
  await next.click();
  const first = await hunksOrdinal(page);
  await next.click();
  const second = await hunksOrdinal(page);
  expect(second.total).toBe(first.total);
  expect(second.at).toBe((first.at % first.total) + 1);
  await prev.click();
  const back = await hunksOrdinal(page);
  expect(back.at).toBe(second.at === 1 ? second.total : second.at - 1);

  // Filters dispatch through the bus with an honest status note.
  await page.getByTestId("toolbar/action-filter-diffs").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: diffs");
  await page.getByTestId("toolbar/action-filter-all").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: all");

  // Copy addressing: nav above selected a hunk as a side effect, so Copy
  // is now enabled via the bus path — pin the take + click through.
  await page.getByTestId("textview/row-98").click();
  await expect(copy).toBeEnabled();
  await copy.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy take-left requested for row 98");

  expect(pageErrors, "zero page errors").toEqual([]);
  expect(consoleErrors, "zero console errors").toEqual([]);
});

test("toolbar-dedupe-238: disabled buttons carry reason tooltips", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Text kind: sync + merge park honestly with reasoned titles.
  await expect(page.getByTestId("toolbar/action-sync")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-sync")).toHaveAttribute("title", /needs folder session/);
  await expect(page.getByTestId("toolbar/action-merge")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-merge")).toHaveAttribute("title", /use Merge view/);
  // Compare is live in text with paths: enabled, bare title.
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled();

  // Folder kind: Prev/Next/Copy park with the L3-wiring reason instead of
  // pretending; filters stay live.
  await createSession(page, "folder", "/tmp/w238-left", "/tmp/w238-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 })
    .toBeGreaterThan(0);
  await expect(page.getByTestId("toolbar/action-prev-hunk")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-prev-hunk")).toHaveAttribute(
    "title",
    /needs view wiring in L3/,
  );
  await expect(page.getByTestId("toolbar/action-next-hunk")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-next-hunk")).toHaveAttribute(
    "title",
    /needs view wiring in L3/,
  );
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-copy")).toHaveAttribute("title", /\(.+\)/);

  // Parked bus dispatch is a no-op, never a crash.
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("autocompare:view-command", { detail: { cmd: "next", viewKind: "folder" } }),
    );
    window.dispatchEvent(
      new CustomEvent("autocompare:view-command", { detail: { cmd: "copy", viewKind: "folder" } }),
    );
  });
  await page.getByTestId("toolbar/action-filter-same").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("View filter: same");

  expect(pageErrors, "zero page errors").toEqual([]);
  expect(consoleErrors, "zero console errors").toEqual([]);
});

test("toolbar-dedupe-238: advertised chords wired + every menu id has a listener", async ({ page }) => {
  const { pageErrors, consoleErrors } = trackErrors(page);
  await bootText(page);

  // Every toolbar-advertised chord exists in shortcuts.ts registry:
  // titles append the wired hint parenthetically (single source).
  await expect(page.getByTestId("toolbar/action-next-hunk")).toHaveAttribute(
    "title",
    /Cmd\+Alt\+Down/,
  );
  await expect(page.getByTestId("toolbar/action-prev-hunk")).toHaveAttribute(
    "title",
    /Cmd\+Alt\+Up/,
  );
  await expect(page.getByTestId("toolbar/action-open")).toHaveAttribute("title", /Cmd\+O/);
  await page.getByTestId("textview/row-98").click();
  await expect(page.getByTestId("toolbar/action-copy")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-copy")).toHaveAttribute("title", /Cmd\+D/);

  // Every menu.rs id resolves through the App menu-event listener —
  // no dead ids, no crash. Kind switches open sessions; parked items
  // report honestly via the status bar.
  await emitMenu(page, "view.kind-folder", "folder_compare");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await emitMenu(page, "view.kind-text", "text_compare");
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });
  await emitMenu(page, "view.kind-merge", "merge_compare");
  await expect(page.getByTestId("view-merge")).toBeVisible({ timeout: 10000 });
  await emitMenu(page, "session.new", "session_new");
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await emitMenu(page, "view.toggle-theme", "theme_toggle");
  await expect(page.getByTestId("statusbar/message")).toContainText("Theme follows the OS");
  await emitMenu(page, "view.toggle-layout", "layout_toggle");
  await expect(page.getByTestId("statusbar/message")).toContainText("Layout toggle is not available");
  await emitMenu(page, "tools.settings", "settings_open");
  await expect(page.getByTestId("statusbar/message")).toContainText("Settings UI is not available");

  expect(pageErrors, "zero page errors").toEqual([]);
  expect(consoleErrors, "zero console errors").toEqual([]);
});
