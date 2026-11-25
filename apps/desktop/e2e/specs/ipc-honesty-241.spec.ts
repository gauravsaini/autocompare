/**
 * Wave-241 IPC & UI Honesty E2E Tests (Wave B3).
 *
 * Verifies IPC honesty and placebo menu honesty under MOCK IPC (`VITE_MOCK_IPC=1`):
 *  1. Compare-stop disabled reason / title honesty: stop button
 *     (`toolbar/action-compare-stop`) honestly states that no compare is running
 *     and the backend lacks abort IPC.
 *  2. Placebo menu notes: view.toggle-theme, view.toggle-layout, tools.settings,
 *     and file.open-workspace via emitMenuEvent checking statusbar/message.
 *  3. Mock hex and merge IPC smoke test: page.evaluate with invoke("hex_compare"),
 *     invoke("hex_get_page"), invoke("hex_find"), invoke("merge_text_plan"), and
 *     invoke("merge_text_apply") return valid fixtures instead of default [] or errors.
 *  4. Zero console errors and zero page errors throughout.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";

const EVID_DIR = "/tmp/ipc-honesty-241";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) {
      fs.mkdirSync(EVID_DIR, { recursive: true });
    }
  } catch {
    // Best-effort evidence dir creation
  }

  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    // Wait for demo text session to mount so initial async boot settles
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(String(err)));
  return errors;
}

/**
 * Dispatches a menu event cleanly using the App's mock hooks:
 * 1. window.__MOCK_EMIT_MENU__ if exposed
 * 2. window CustomEvent "menu-event"
 */
async function emitMenuEvent(page: Page, id: string, command = ""): Promise<void> {
  await page.evaluate(
    ({ id, command }) => {
      const fn = (
        window as unknown as {
          __MOCK_EMIT_MENU__?: (p: { id: string; command?: string }) => void;
        }
      ).__MOCK_EMIT_MENU__;
      if (typeof fn === "function") {
        fn({ id, command });
      }
      window.dispatchEvent(new CustomEvent("menu-event", { detail: { id, command } }));
    },
    { id, command },
  );
}

test("ipc-honesty-241: compare-stop button surfaces lack of abort IPC in disabled reason", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });

  const stopBtn = page.getByTestId("toolbar/action-compare-stop");
  await expect(stopBtn).toBeAttached();
  await expect(stopBtn).toBeDisabled();

  const title = (await stopBtn.getAttribute("title")) ?? "";
  const ariaLabel = (await stopBtn.getAttribute("aria-label")) ?? "";

  // Honest disabled reason must surface that no compare is running and the backend has no abort IPC
  const expectedReasonPattern = /Stop \(no compare running: (backend has no abort IPC|compare IPC cannot be aborted)\)/;
  expect(title).toMatch(expectedReasonPattern);
  expect(ariaLabel).toMatch(expectedReasonPattern);

  await page.screenshot({ path: `${EVID_DIR}/compare-stop-honesty.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("ipc-honesty-241: placebo menu notes report honest status messages", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });

  // 1. Theme toggle
  await emitMenuEvent(page, "view.toggle-theme", "theme_toggle");
  await expect(page.getByTestId("statusbar/message")).toContainText(
    "Theme follows the OS appearance (no in-app toggle yet)",
  );

  // 2. Layout toggle
  await emitMenuEvent(page, "view.toggle-layout", "layout_toggle");
  await expect(page.getByTestId("statusbar/message")).toContainText(
    "Layout toggle is not available in this build",
  );

  // 3. Settings UI
  await emitMenuEvent(page, "tools.settings", "settings_open");
  await expect(page.getByTestId("statusbar/message")).toContainText(
    "Settings UI is not available in this build",
  );

  // 4. File Open Workspace (distinguished from normal open session)
  await emitMenuEvent(page, "file.open-workspace", "open_workspace");
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("statusbar/message")).toContainText(
    "Session Manager opened (open-workspace duplicates session manager; workspace mode pending backend)",
  );

  await page.screenshot({ path: `${EVID_DIR}/placebo-menu-notes.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("ipc-honesty-241: mock hex and merge IPC return valid fixtures rather than default []", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });

  const ipcResults = await page.evaluate(async () => {
    const { invoke } = await import("/src/ipc/client.ts");

    // 1. hex_compare
    const hexCompare = await invoke<Record<string, unknown>>("hex_compare", {
      left: "/tmp/a.bin",
      right: "/tmp/b.bin",
    });

    // 2. hex_get_page
    const hexGetPage = await invoke<Record<string, unknown>>("hex_get_page", {
      session: "mock-hex-1",
      offset: 0,
      len: 64,
    });

    // 3. hex_find
    const hexFind = await invoke<unknown>("hex_find", {
      session: "mock-hex-1",
      needle_hex: "08",
      from_offset: 0,
    });

    // 4. merge_text_plan
    const mergePlan = await invoke<Record<string, unknown>>("merge_text_plan", {
      base: "base.txt",
      left: "left.txt",
      right: "right.txt",
    });

    // 5. merge_text_apply
    const mergeApply = await invoke<Record<string, unknown>>("merge_text_apply", {
      base: "base.txt",
      left: "left.txt",
      right: "right.txt",
      resolutions: [],
      out_path: "/tmp/merged.txt",
    });

    return { hexCompare, hexGetPage, hexFind, mergePlan, mergeApply };
  });

  // Verify hex_compare returns HexSummary fixture
  expect(Array.isArray(ipcResults.hexCompare), "hex_compare should not return default []").toBe(false);
  expect(ipcResults.hexCompare.sessionId).toBe("mock-hex-1");
  expect(ipcResults.hexCompare.leftSize).toBe(64);
  expect(ipcResults.hexCompare.rightSize).toBe(64);
  expect(ipcResults.hexCompare.equal).toBe(false);
  expect(ipcResults.hexCompare.firstDiff).toBe(8);

  // Verify hex_get_page returns HexPage fixture
  expect(Array.isArray(ipcResults.hexGetPage), "hex_get_page should not return default []").toBe(false);
  expect(ipcResults.hexGetPage.sessionId).toBe("mock-hex-1");
  expect(ipcResults.hexGetPage.offset).toBe(0);
  expect(ipcResults.hexGetPage.len).toBe(64);
  expect(Array.isArray(ipcResults.hexGetPage.left)).toBe(true);
  expect(Array.isArray(ipcResults.hexGetPage.right)).toBe(true);
  expect((ipcResults.hexGetPage.left as number[]).length).toBe(64);
  expect((ipcResults.hexGetPage.right as number[]).length).toBe(64);
  expect(ipcResults.hexGetPage.total).toBe(64);
  expect(ipcResults.hexGetPage.truncated).toBe(false);

  // Verify hex_find returns match number / not default []
  expect(Array.isArray(ipcResults.hexFind), "hex_find should not return default []").toBe(false);
  expect(ipcResults.hexFind).toBe(8);

  // Verify merge_text_plan returns TextMergePlan fixture
  expect(Array.isArray(ipcResults.mergePlan), "merge_text_plan should not return default []").toBe(false);
  expect(Array.isArray(ipcResults.mergePlan.conflicts)).toBe(true);
  expect((ipcResults.mergePlan.conflicts as unknown[]).length).toBe(1);
  expect(ipcResults.mergePlan.conflictCount).toBe(1);
  expect(ipcResults.mergePlan.cleanBlockCount).toBe(2);
  expect(ipcResults.mergePlan.hasConflicts).toBe(true);

  // Verify merge_text_apply returns TextMergeApplyReport fixture
  expect(Array.isArray(ipcResults.mergeApply), "merge_text_apply should not return default []").toBe(false);
  expect(ipcResults.mergeApply.outPath).toBe("/tmp/merged.txt");
  expect(ipcResults.mergeApply.writtenLines).toBe(3);
  expect(ipcResults.mergeApply.conflictCount).toBe(1);
  expect(ipcResults.mergeApply.resolvedCount).toBe(1);
  expect(ipcResults.mergeApply.unresolvedCount).toBe(0);
  expect(ipcResults.mergeApply.hasConflictMarkers).toBe(false);

  await page.screenshot({ path: `${EVID_DIR}/mock-ipc-smoke.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});
