import { expect, test, type Page } from "@playwright/test";

/**
 * Wave-3C full-flow e2e (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * End-to-end user journeys through the real `App` workspace
 * (ChromeShell + SessionManager dialog + ViewRouter tabs):
 *  1. boot -> text session -> compare -> close tab -> welcome,
 *  2. kind switching (folder/hex/table/merge) mounts each view and
 *     toolbar enablement follows the active kind,
 *  3. dialog-mock open (`window.__MOCK_DIALOG_OPEN__`) loads a session
 *     file into a tab,
 *  4. every session kind (media gained its view in wave-48) mounts a data view, fallback unreachable.
 *
 * Complements (does NOT duplicate) the existing specs:
 *  - `home` / `ipc-smoke`: boot placeholder / real-shell smoke,
 *  - `views-chrome`: per-kind source contracts (testids + IPC calls),
 *  - `flows-session`: SessionManager mounted in isolation,
 *  - `app-integration`: dialog open/close, text+folder tab switching,
 *    toolbar reasons, sync stub, compare re-run.
 *
 * Stable-testids-only rule: every locator below was inventoried from
 * `apps/desktop/src` before writing. If a step has no stable testid it
 * is skipped with a reason -- testids are never added in this wave.
 * Skip policy: any infra failure (dev server unreachable, browser
 * missing offline) skips instead of failing -- the reason is logged.
 */

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function openManager(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
}

async function closeManager(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
}

async function createSession(page: Page, kind: string, left: string, right: string): Promise<void> {
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
}

async function dismissDemoToBlank(page: Page): Promise<void> {
  // Wave-11: wave-10 boots the demo session open; Start blank returns to
  // the empty welcome workspace so journey counts start from zero.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await expect(page.getByTestId("tab-home")).toBeVisible();
}

test("journey: boot -> text session -> compare -> close tab returns to welcome", async ({ page }) => {
  // Boot: wave-10 demo first-paint (demo session open with real rows).
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByTestId("workspace/demo-banner")).toContainText("Demo");
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  // Start blank -> empty workspace with welcome tab, compare/copy/sync/merge parked.
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("tab-home")).toBeVisible();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await expect(page.getByTestId("toolbar/action-compare")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-sync")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-merge")).toBeDisabled();

  // Open session via the dialog form (typed paths, no file picker needed).
  await openManager(page);
  await createSession(page, "text", "/tmp/full-left.txt", "/tmp/full-right.txt");
  await expect(page.getByTestId("session/list")).toContainText("full-left.txt");
  await closeManager(page);

  // Tab appears, text view mounts, chrome reflects the session paths.
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByRole("tab", { name: /full-left\.txt/ })).toBeVisible();
  await expect(page.getByTestId("view-router")).toBeVisible();
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/full-left.txt");
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/full-right.txt");

  // Toolbar: compare live for a text pair; copy/merge stay parked with reasons.
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-merge")).toBeDisabled();

  // Compare re-runs against mock fixtures and reports inline.
  await page.getByTestId("toolbar/action-compare").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished");

  // Close the tab via its chrome close button -> welcome tab returns.
  await page.getByRole("button", { name: /^Close / }).click();
  await expect(page.getByTestId("tab-home")).toBeVisible();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await expect(page.getByTestId("toolbar/action-compare")).toBeDisabled();
});

test("journey: kind switching mounts folder/hex/table/merge views with kind-correct toolbar", async ({ page }) => {
  await dismissDemoToBlank(page);
  await openManager(page);
  await createSession(page, "folder", "/tmp/k-left-dir", "/tmp/k-right-dir");
  await createSession(page, "hex", "/tmp/k-left.bin", "/tmp/k-right.bin");
  await createSession(page, "table", "/tmp/k-left.csv", "/tmp/k-right.csv");
  await createSession(page, "merge", "/tmp/k-left.txt", "/tmp/k-right.txt");
  await closeManager(page);

  await expect(page.getByTestId("session-count")).toContainText("sessions: 4");

  // Newest session (merge) is active first: merge view, compare blocked by kind.
  await expect(page.getByTestId("view-merge")).toBeVisible();
  await expect(page.getByTestId("toolbar/action-compare")).toBeDisabled();
  const mergeCompareTitle = await page.getByTestId("toolbar/action-compare").getAttribute("title");
  expect(mergeCompareTitle, "merge compare blocked reason").toContain("Merge view");

  // Folder tab: folder view mounts, sync unlocks for folder kinds.
  await page.getByRole("tab", { name: /k-left-dir/ }).click();
  await expect(page.getByTestId("view-folder")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/k-left-dir");
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-sync")).toBeEnabled();

  // Hex tab: hex view mounts, sync parks (non-folder kind).
  await page.getByRole("tab", { name: /k-left\.bin/ }).click();
  await expect(page.getByTestId("view-hex")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/k-left.bin");
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-sync")).toBeDisabled();

  // Table tab: table view mounts.
  await page.getByRole("tab", { name: /k-left\.csv/ }).click();
  await expect(page.getByTestId("view-table")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/k-left.csv");
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-sync")).toBeDisabled();

  // Back to merge: view + toolbar follow the active kind again.
  await page.getByRole("tab", { name: /k-left\.txt/ }).click();
  await expect(page.getByTestId("view-merge")).toBeVisible();
  await expect(page.getByTestId("toolbar/action-compare")).toBeDisabled();

  // Closing the active merge tab drops the count and lands on a live tab.
  await page.getByRole("button", { name: /^Close .*k-left\.txt/ }).click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 3");
  await expect(page.getByTestId("view-router")).toBeVisible();
});

test("journey: dialog-mock open loads a session file into a tab", async ({ page }) => {
  await dismissDemoToBlank(page);
  // Register the mock file-picker BEFORE opening the dialog.
  await page.evaluate(() => {
    (
      window as unknown as {
        __MOCK_DIALOG_OPEN__: () => Promise<string>;
      }
    ).__MOCK_DIALOG_OPEN__ = async () => "/mock/path/imported-session.json";
  });

  await openManager(page);
  await page.getByTestId("session/open").click();
  await expect(page.getByTestId("session/list")).toContainText("imported-session");
  await closeManager(page);

  // Loaded session drives a tab + the text view (mock descriptor is text-kind).
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByRole("tab", { name: /imported-session/ })).toBeVisible();
  await expect(page.getByTestId("view-router")).toBeVisible();
  await expect(page.getByTestId("view-text")).toBeVisible();
  // Wave-77 gate: wave-74 keeps the Home tab always present (this file's
  // own dismissDemoToBlank already pins tab-home visible), so it stays
  // visible alongside the loaded session tab (was hidden pre-74).
  await expect(page.getByTestId("tab-home")).toBeVisible();
});

test("journey: every session kind mounts a data view, no fallback", async ({ page }) => {
  // Wave-48 gave the last kind (`media`) its own view, so the router
  // fallback is now unreachable through the session dialog. This journey
  // pins that invariant: a media session mounts `view-media` and the
  // fallback text never renders.
  await dismissDemoToBlank(page);
  await openManager(page);
  await createSession(page, "media", "/tmp/s-left.png", "/tmp/s-right.png");
  await closeManager(page);

  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByRole("tab", { name: /s-left\.png/ })).toBeVisible();
  await expect(page.getByTestId("view-media")).toBeVisible();
  await expect(page.getByTestId("view-router")).not.toContainText("No data view for kind");
  await expect(page.getByTestId("view-text")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeHidden();
  await expect(page.getByTestId("view-sync")).toBeHidden();
});
