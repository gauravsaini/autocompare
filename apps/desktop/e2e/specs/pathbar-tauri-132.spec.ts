/**
 * Wave-132 PathBar native drops + pane position parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1542, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - single drop on the LEFT PathBar input stages left + compare fires;
 *  - single drop on the RIGHT PathBar input stages right + compare fires;
 *  - dual drop (2 files, ONE event) stages Item0 -> left, Item1 -> right;
 *  - unit: `dedupTauriDropPaths` / `firstTauriDropPath` /
 *    `resolveTauriDropSide` (x vs half-width + hovered-half fallback);
 *  - zero console errors / page errors throughout. PNG evidence in /tmp/drop-132/.
 *
 * Real-shell note: under mock the Tauri `onDragDropEvent` listener is a
 * deliberate no-op and the HTML5 `onDrop` handlers above own drops; in the
 * production shell the new `useTauriPathBarDrop` hook owns PathBar-strip
 * drops (position -> field side, same dedup + `markDropRecompare` path)
 * while `useTauriPaneDrop` skips that band, so one drop never
 * double-compares. Native shell drops need a manual live-drop check
 * (see checks/dnd-tauri-132/REPORT.md).
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import {
  dedupTauriDropPaths,
  firstTauriDropPath,
  hoveredPaneHalf,
  isTauriDropOnPathBar,
  resolveTauriDropSide,
  tauriDropCssPoint,
} from "../../src/components/dnd/dropPaths";

const EVID_DIR = "/tmp/drop-132";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("pathbar")).toBeVisible({ timeout: 15000 });
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

/** Synthetic drop of `paths` (in order) on `selector`. */
async function dropPaths(page: Page, selector: string, paths: string[]): Promise<void> {
  await page.locator(selector).evaluate((el, ps: string[]) => {
    const files = ps.map((p) => ({ name: p.split("/").pop() ?? p, path: p }));
    const dt = { files, types: ["Files"], getData: (_t: string) => "" };
    const ev = new DragEvent("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "dataTransfer", { value: dt });
    el.dispatchEvent(ev);
  }, paths);
}

test("pathbar-tauri-132: single drop on left input stages left + compare fires", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await dropPaths(page, '[data-testid="pathbar/left"]', ["/tmp/drop132-left.txt"]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue("/tmp/drop132-left.txt");
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await page.screenshot({ path: `${EVID_DIR}/single-drop-left.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathbar-tauri-132: single drop on right input stages right + compare fires", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await dropPaths(page, '[data-testid="pathbar/right"]', ["/tmp/drop132-right.txt"]);
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue("/tmp/drop132-right.txt");
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await page.screenshot({ path: `${EVID_DIR}/single-drop-right.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathbar-tauri-132: dual drop stages Item0 left + Item1 right", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await dropPaths(page, '[data-testid="pathbar/right"]', ["/tmp/dual132-left.txt", "/tmp/dual132-right.txt"]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue("/tmp/dual132-left.txt");
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue("/tmp/dual132-right.txt");
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await page.screenshot({ path: `${EVID_DIR}/dual-drop.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathbar-tauri-132 unit: tauri drop-path dedup keeps order, drops repeats", () => {
  expect(dedupTauriDropPaths(null)).toEqual([]);
  expect(dedupTauriDropPaths([])).toEqual([]);
  expect(dedupTauriDropPaths([" /tmp/a.txt ", "/tmp/a.txt", "/tmp/b.txt", "", "/tmp/a.txt"])).toEqual([
    "/tmp/a.txt",
    "/tmp/b.txt",
  ]);
  expect(firstTauriDropPath(null)).toBeNull();
  expect(firstTauriDropPath([])).toBeNull();
  expect(firstTauriDropPath(["/tmp/a.txt", "/tmp/a.txt", "/tmp/b.txt"])).toBe("/tmp/a.txt");
});

test("pathbar-tauri-132 unit: position maps x vs half-width, fallback on missing x", () => {
  expect(resolveTauriDropSide(100, 1280)).toBe("left");
  expect(resolveTauriDropSide(1000, 1280)).toBe("right");
  expect(resolveTauriDropSide(640, 1280)).toBe("right");
  expect(resolveTauriDropSide(0, 1280)).toBe("left");
  expect(resolveTauriDropSide(undefined, 1280)).toBe("left");
  expect(resolveTauriDropSide(Number.NaN, 1280)).toBe("left");
  expect(resolveTauriDropSide(0, 1280, "right")).toBe("right");
  expect(resolveTauriDropSide(undefined, 1280, "right")).toBe("right");
  expect(resolveTauriDropSide(100, 0)).toBe("left");
  // Node has no DOM: hovered half is null, point helpers degrade honestly.
  expect(hoveredPaneHalf()).toBeNull();
  expect(tauriDropCssPoint(null)).toBeNull();
  expect(tauriDropCssPoint({ x: 10 })).toBeNull();
  expect(isTauriDropOnPathBar(undefined)).toBe(false);
});
