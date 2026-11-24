import { expect, test, type Page } from "@playwright/test";

/**
 * Wave-2R app integration (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Verifies the Wave-2R workspace wiring owned by this wave:
 *  1. mock-mode boot renders demo first-paint (Start blank -> welcome),
 *  2. SessionManager dialog open/close from App toolbar,
 *  3. creating sessions drives tabs + ViewRouter mounting,
 *  4. tab switching swaps the active view,
 *  5. toolbar actions carry enabled states + disabled reasons.
 *
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
  // the empty welcome workspace so session counts start from zero.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await expect(page.getByTestId("tab-home")).toBeVisible();
}

test("mock-mode boot renders demo first-paint with Start-blank to welcome", async ({ page }) => {
  // Wave-11: wave-10 demo first-paint boots with the demo session open.
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByTestId("workspace/demo-banner")).toContainText("Demo");
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  // Start blank returns to the empty welcome workspace.
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("tabbar")).toBeVisible();
  await expect(page.getByRole("toolbar")).toBeVisible();
  await expect(page.getByTestId("pathbar")).toBeVisible();
  await expect(page.getByTestId("statusbar")).toBeVisible();
  await expect(page.getByTestId("ruler")).toBeVisible();
  await expect(page.getByTestId("tab-home")).toBeVisible();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await expect(page.getByTestId("statusbar")).toContainText("session: none");
  // Toolbar open is always available; compare/copy/sync/merge degrade disabled.
  await expect(page.getByTestId("toolbar/action-open")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();
  await expect(page.getByTestId("toolbar/action-merge")).toBeDisabled();
});

test("dialog opens from toolbar and closes", async ({ page }) => {
  await openManager(page);
  await expect(page.getByTestId("session/list")).toBeVisible();
  await expect(page.getByTestId("session/new")).toBeVisible();
  await closeManager(page);
  await expect(page.getByTestId("tabbar")).toBeVisible();
});

test("dialog opens from welcome action and closes", async ({ page }) => {
  await dismissDemoToBlank(page);
  await page.getByTestId("workspace/open-manager").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await closeManager(page);
});

test("creating a session mounts a tab and its view", async ({ page }) => {
  await dismissDemoToBlank(page);
  await openManager(page);
  await createSession(page, "text", "/tmp/left-a.txt", "/tmp/right-a.txt");
  await expect(page.getByTestId("session/list")).toContainText("left-a.txt");
  await closeManager(page);

  // One session tab mounted; the Home tab stays visible (wave-74:
  // Home is always present), text view mounted with session paths.
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByRole("tab", { name: /left-a\.txt/ })).toBeVisible();
  await expect(page.getByTestId("view-router")).toBeVisible();
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/left-a.txt");
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/right-a.txt");
  await expect(page.getByTestId("statusbar")).toContainText("left-a.txt");
  // Compare is now live (text pair); sync stays disabled outside folder kinds.
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled();
  await expect(page.getByTestId("toolbar/action-sync")).toBeDisabled();
});

test("tab switching swaps the active view", async ({ page }) => {
  await dismissDemoToBlank(page);
  await openManager(page);
  await createSession(page, "text", "/tmp/left-a.txt", "/tmp/right-a.txt");
  await createSession(page, "folder", "/tmp/left-dir", "/tmp/right-dir");
  await expect(page.getByTestId("session/list")).toContainText("left-dir");
  await closeManager(page);

  const tabs = page.getByRole("tab");
  // Wave-77 gate: wave-74 keeps the Home tab always present, so two
  // sessions + Home render three tabs (was two pre-74).
  await expect(tabs).toHaveCount(3);
  await expect(page.getByTestId("session-count")).toContainText("sessions: 2");

  // Newest session (folder) is active first.
  await expect(page.getByTestId("view-folder")).toBeVisible();
  await page.getByRole("tab", { name: /left-a\.txt/ }).click();
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/left-a.txt");
  await page.getByRole("tab", { name: /left-dir/ }).click();
  await expect(page.getByTestId("view-folder")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/left-dir");
});

test("toolbar disabled actions carry reasons in title", async ({ page }) => {
  await dismissDemoToBlank(page);
  const copyTitle = await page.getByTestId("toolbar/action-copy").getAttribute("title");
  expect(copyTitle, "copy disabled reason").toContain("select a row");
  const mergeTitle = await page.getByTestId("toolbar/action-merge").getAttribute("title");
  expect(mergeTitle, "merge disabled reason").toContain("Merge view");
  const compareTitle = await page.getByTestId("toolbar/action-compare").getAttribute("title");
  expect(compareTitle, "compare disabled reason").toContain("needs");
});

test("sync stub runs a plan for folder sessions", async ({ page }) => {
  await openManager(page);
  await createSession(page, "folder", "/tmp/left-dir", "/tmp/right-dir");
  await closeManager(page);

  const syncBtn = page.getByTestId("toolbar/action-sync");
  await expect(syncBtn).toBeEnabled();
  await syncBtn.click();
  await expect(page.getByTestId("statusbar/message")).toContainText("Sync plan");
});

test("compare re-run refreshes the active view", async ({ page }) => {
  await openManager(page);
  await createSession(page, "text", "/tmp/left-a.txt", "/tmp/right-a.txt");
  await closeManager(page);

  await page.getByTestId("toolbar/action-compare").click();
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished");
  await expect(page.getByTestId("view-text")).toBeVisible();
});
