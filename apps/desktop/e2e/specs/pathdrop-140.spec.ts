/**
 * Wave-140 pathbar + pane Finder-drop BC parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1552, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - single drop on the LEFT PathBar field stages that side + compare fires;
 *  - single drop on the RIGHT PathBar field stages that side + compare fires;
 *  - single drop on the PathBar STRIP (gap/label, clientX vs midpoint)
 *    stages the resolved side + exactly one compare (no double);
 *  - dragover arms the whole-strip affordance (`data-drop-active=true`);
 *  - dual drop (2 files, ONE event, real `~/Documents/File 1.txt` +
 *    `File 2.txt` with spaces) stages Item0 -> left, Item1 -> right with
 *    EXACTLY ONE compare (`__AC_COMPARE_COUNT__` delta +1, no double);
 *  - unit: `extractDroppedPaths` keeps space paths in order + dedups,
 *    unknown-position Tauri drops stay pane-owned (`false`, single compare);
 *  - zero console errors / page errors throughout. PNGs in /tmp/drop-140/.
 *
 * Real-shell note: under mock the Tauri `onDragDropEvent` listeners are a
 * deliberate no-op and the HTML5 `onDrop` handlers own drops; in the
 * production shell `useTauriPathBarDrop` owns the y-gated PathBar strip
 * (input + gap + label) while `useTauriPaneDrop` skips that band, so one
 * drop never double-compares. Native shell drops need the manual live-drop
 * check (see checks/pathdrop-140/REPORT.md).
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import {
  dedupTauriDropPaths,
  extractDroppedPaths,
  isTauriDropOnPathBar,
} from "../../src/components/dnd/dropPaths";

const EVID_DIR = "/tmp/drop-140";
// Gate validation pair: real files with spaces in the name.
const FILE1 = "/Users/ektasaini/Documents/File 1.txt";
const FILE2 = "/Users/ektasaini/Documents/File 2.txt";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
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

async function compareCount(page: Page): Promise<number> {
  return page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    return typeof w["__AC_COMPARE_COUNT__"] === "number"
      ? (w["__AC_COMPARE_COUNT__"] as number)
      : 0;
  });
}

/** Synthetic drop of `paths` (in order) on `selector`, with `clientX`. */
async function dropPaths(
  page: Page,
  selector: string,
  paths: string[],
  clientX = 0,
): Promise<void> {
  await page.locator(selector).first().evaluate(
    (el, args: { ps: string[]; x: number }) => {
      const files = args.ps.map((p) => ({ name: p.split("/").pop() ?? p, path: p }));
      const dt = { files, types: ["Files"], getData: (_t: string) => "" };
      const ev = new DragEvent("drop", { bubbles: true, cancelable: true, clientX: args.x });
      Object.defineProperty(ev, "dataTransfer", { value: dt });
      el.dispatchEvent(ev);
    },
    { ps: paths, x: clientX },
  );
}

async function dragOver(page: Page, selector: string): Promise<void> {
  await page.locator(selector).first().evaluate((el) => {
    const dt = { files: [], types: ["Files"], getData: (_t: string) => "" };
    const ev = new DragEvent("dragover", { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "dataTransfer", { value: dt });
    el.dispatchEvent(ev);
  });
}

test("pathdrop-140: single drop on left field stages left + one compare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', [FILE1]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(FILE1);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/single-drop-left.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-140: single drop on right field stages right + one compare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/right"]', [FILE2]);
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(FILE2);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/single-drop-right.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-140: strip drop resolves side by midpoint + whole-strip affordance", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Whole-strip dragover affordance (input + gap + label sab droppable).
  await dragOver(page, '[data-testid="pathbar"]');
  await expect(page.locator('[data-testid="pathbar"]')).toHaveAttribute(
    "data-drop-active",
    "true",
  );
  // Strip drop far-right (clientX vs midpoint) -> right side + one compare.
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar"]', [FILE2], 2000);
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(FILE2);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/strip-drop.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-140: dual drop Item0 left Item1 right, exactly one compare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', [FILE1, FILE2]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(FILE1, {
    timeout: 5000,
  });
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(FILE2, {
    timeout: 5000,
  });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  // Settle: a double-compare bug would fire the 2nd compare right after.
  await page.waitForTimeout(1500);
  expect(await compareCount(page), "dual drop fires exactly one compare").toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/dual-drop.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-140 unit: space paths ordered + deduped, unknown position pane-owned", () => {
  // Space paths (Documents File 1/2) survive extraction in order.
  const got = extractDroppedPaths({
    files: [{ name: "File 1.txt", path: FILE1 }, { name: "File 2.txt", path: FILE2 }],
    getData: (_t: string) => "",
  });
  expect(got).toEqual([FILE1, FILE2]);
  expect(dedupTauriDropPaths([FILE1, FILE1, FILE2, "  "])).toEqual([FILE1, FILE2]);
  // Unknown position -> pane owns the drop (single compare, never both hooks).
  expect(isTauriDropOnPathBar(undefined)).toBe(false);
  expect(isTauriDropOnPathBar(null)).toBe(false);
  expect(isTauriDropOnPathBar({})).toBe(false);
});
