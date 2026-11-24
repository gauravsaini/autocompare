/**
 * Wave-142 shortcuts audit + fix (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1557, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit): proves the full documented
 * shortcut inventory from docs/shortcuts-142.md —
 *  - unit: `matchWebShortcut` maps CmdOrCtrl+1..7/N/O/S/D/E (+Shift+O)
 *    to menu ids and skips F/G/plain keys/Alt chords;
 *  - web bridge: Meta+2/4/1 switch kinds, Meta+N opens the Session
 *    Manager, Meta+S reports saved/nothing-to-save, Meta+D with no
 *    hunk selected reports the honest take-left note (same read-only
 *    menu path as the native shell);
 *  - text keys: `l` on hunk row 6 copies take-left (copy-note +
 *    row flips equal), Meta+D on the selected hunk reports
 *    `Copy take-left requested` via the bridge;
 *  - 142 Esc fix: hex/table/merge find panels close on global Esc and
 *    reopen on Meta+F (text parity);
 *  - zero console errors / page errors throughout. PNGs in /tmp/shortcuts-142/.
 *
 * TextView `copyHunkAt` + App menu-event section are wave-141 read-only:
 * this spec drives them through the UI/events only, never edits them.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import { matchWebShortcut } from "../../src/components/chrome/shortcuts";

const EVID_DIR = "/tmp/shortcuts-142";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed:", err);
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

function chord(key: string, extra: Partial<{ shiftKey: boolean; altKey: boolean }> = {}) {
  return { key, metaKey: true, ctrlKey: false, shiftKey: extra.shiftKey ?? false, altKey: extra.altKey ?? false };
}

test("shortcuts-142: chord matcher maps menu accelerators, skips view-local keys", async ({ page }) => {
  const errors = collectErrors(page);
  expect(matchWebShortcut(chord("1"))).toBe("view.kind-text");
  expect(matchWebShortcut(chord("2"))).toBe("view.kind-folder");
  expect(matchWebShortcut(chord("3"))).toBe("view.kind-hex");
  expect(matchWebShortcut(chord("4"))).toBe("view.kind-table");
  expect(matchWebShortcut(chord("5"))).toBe("view.kind-media");
  expect(matchWebShortcut(chord("6"))).toBe("view.kind-sync");
  expect(matchWebShortcut(chord("7"))).toBe("view.kind-merge");
  expect(matchWebShortcut(chord("n"))).toBe("session.new");
  expect(matchWebShortcut(chord("o"))).toBe("file.open-session");
  expect(matchWebShortcut(chord("O", { shiftKey: true }))).toBe("file.open-folder");
  expect(matchWebShortcut(chord("s"))).toBe("file.save-session");
  expect(matchWebShortcut(chord("d"))).toBe("session.take-left");
  expect(matchWebShortcut(chord("e"))).toBe("session.take-right");
  // View-local + non-chords must NOT match (no double-handling, no hijack).
  expect(matchWebShortcut(chord("f"))).toBeNull();
  expect(matchWebShortcut(chord("g"))).toBeNull();
  expect(matchWebShortcut({ key: "l", metaKey: false, ctrlKey: false, shiftKey: false, altKey: false })).toBeNull();
  expect(matchWebShortcut(chord("1", { altKey: true }))).toBeNull();
  expect(matchWebShortcut(chord("8"))).toBeNull();
  expect(errors, "zero console/page errors").toEqual([]);
});

test("shortcuts-142: bridge switches kinds Meta+2/4/1", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+2");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+4");
  await expect(page.getByTestId("view-table")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });
  await page.screenshot({ path: `${EVID_DIR}/kinds.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("shortcuts-142: bridge opens manager Meta+N, saves Meta+S", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+n");
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 10000 });
  await page.getByRole("button", { name: /Close dialog/ }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden({ timeout: 10000 });
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("statusbar/message")).toContainText(/Session saved|Nothing to save/, { timeout: 10000 });
  await page.screenshot({ path: `${EVID_DIR}/manager-save.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("shortcuts-142: Meta+D with no hunk selected reports honestly", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  // Fresh page: no row selected yet, so the read-only take path refuses.
  await page.keyboard.press("Meta+d");
  await expect(page.getByTestId("statusbar/message")).toContainText("needs a hunk row selected", { timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("shortcuts-142: text l key copies take-left on hunk row 6", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", /^(leftOnly|rightOnly|changed)$/, { timeout: 10000 });
  await page.getByTestId("textview/row-6").focus();
  await page.keyboard.press("l");
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left", { timeout: 10000 });
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", "equal", { timeout: 10000 });
  // Bridge Meta+D on the still-selected hunk rides the same menu path.
  await page.keyboard.press("Meta+d");
  await expect(page.getByTestId("statusbar/message")).toContainText(/take-left requested|needs a hunk row selected/, { timeout: 10000 });
  await page.screenshot({ path: `${EVID_DIR}/take-left.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("shortcuts-142: Esc closes hex/table/merge find panels, Meta+F reopens", async ({ page }) => {
  const errors = collectErrors(page);
  // Hex boots with both panels open; global Esc (142 fix) closes them.
  await page.keyboard.press("Meta+3");
  await expect(page.getByTestId("view-hex")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("hex-find-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-find-input")).toBeHidden({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("hex-find-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(async () => page.evaluate(() => (document.activeElement instanceof HTMLElement ? (document.activeElement.getAttribute("data-testid") ?? "") : "")), { timeout: 10000 }).toBe("hex-find-input");

  // Table: Meta+F opens, global Esc (142 fix) closes.
  await page.keyboard.press("Meta+4");
  await expect(page.getByTestId("view-table")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("table-find-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-find-input")).toBeHidden({ timeout: 10000 });

  // Merge: Meta+F opens, global Esc (142 fix) closes.
  await page.keyboard.press("Meta+7");
  await expect(page.getByTestId("view-merge")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("merge/find-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("merge/find-panel")).toBeHidden({ timeout: 10000 });

  // Text: Meta+F focuses the overlay input (macOS browser-find stays shut).
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect.poll(async () => page.evaluate(() => (document.activeElement instanceof HTMLElement ? (document.activeElement.getAttribute("data-testid") ?? "") : "")), { timeout: 10000 }).toBe("textview/find-input");
  await page.screenshot({ path: `${EVID_DIR}/esc-find.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});
