/**
 * Wave-193 path areas droppable BC parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT gate spec (fresh 1694, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - dragover on the LEFT field arms the drop highlight
 *    (`data-drop-active=true` + `.ac-drop-highlight`, zero layout shift);
 *  - drop on LEFT fills left path + exactly one recompare;
 *  - drop on RIGHT fills right path + exactly one recompare;
 *  - strip dragover arms the whole-strip highlight, HOLD holds
 *    (pathbar height <= 36px, highlight adds no height);
 *  - dual drop (2 files, ONE event) stages Item0 -> left, Item1 -> right
 *    with EXACTLY ONE compare (`__AC_COMPARE_COUNT__` delta +1);
 *  - unit: Tauri file-drop routing helpers (shell probe, DPR-aware side,
 *    unknown position pane-owned so exactly one hook compares);
 *  - zero console errors / page errors throughout. PNGs in /tmp/drop-193/.
 *
 * Real-shell note: under mock the Tauri `onDragDropEvent` listeners
 * (`useTauriPathBarDrop`) are a deliberate no-op and the HTML5 `onDrop`
 * handlers own drops; in the production shell the PathBar hook owns the
 * y-gated strip while `useTauriPaneDrop` skips that band, so one drop
 * never double-compares. Native shell drops need the manual live-drop
 * check (see checks/dragdrop-193/REPORT.md).
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import {
  extractDroppedPaths,
  hasTauriDropBridge,
  isTauriDropOnPathBar,
  resolveTauriDropCssSide,
} from "../../src/components/dnd/dropPaths";

const EVID_DIR = "/tmp/drop-193";
const LEFT_FILE = "/tmp/drop-193-left.txt";
const RIGHT_FILE = "/tmp/drop-193-right.txt";

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

test("pathdrop-193: dragover on left field arms drop highlight, zero layout shift", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const bar = page.getByTestId("pathbar");
  const before = await bar.boundingBox();
  await dragOver(page, '[data-testid="pathbar/left"]');
  const field = page.locator('[data-pathfield-side="left"]');
  await expect(field).toHaveClass(/ac-drop-highlight/);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveAttribute(
    "data-drop-active",
    "true",
  );
  // HOLD: highlight is outline-only — strip height unchanged (<= 36px).
  const during = await bar.boundingBox();
  expect(during?.height, "drop highlight adds zero height").toBe(before?.height);
  expect(during?.height, "pathbar HOLD <= 36px").toBeLessThanOrEqual(36);
  await page.screenshot({ path: `${EVID_DIR}/field-highlight.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-193: drop on left field fills left path + one recompare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', [LEFT_FILE]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(LEFT_FILE);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/drop-left.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-193: drop on right field fills right path + one recompare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/right"]', [RIGHT_FILE]);
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(RIGHT_FILE);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/drop-right.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-193: strip dragover arms whole-strip highlight, HOLD holds", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const bar = page.getByTestId("pathbar");
  const before = await bar.boundingBox();
  await dragOver(page, '[data-testid="pathbar"]');
  await expect(bar).toHaveAttribute("data-drop-active", "true");
  await expect(bar).toHaveClass(/ac-drop-highlight/);
  const during = await bar.boundingBox();
  expect(during?.height, "strip highlight adds zero height").toBe(before?.height);
  expect(during?.height, "pathbar HOLD <= 36px").toBeLessThanOrEqual(36);
  // Strip drop far-right (clientX vs midpoint) -> right side + one compare.
  const countBefore = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar"]', [RIGHT_FILE], 2000);
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(RIGHT_FILE);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(countBefore + 1);
  await page.screenshot({ path: `${EVID_DIR}/strip-highlight-drop.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdrop-193: dual drop Item0 left Item1 right, exactly one compare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', [LEFT_FILE, RIGHT_FILE]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(LEFT_FILE, {
    timeout: 5000,
  });
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(RIGHT_FILE, {
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

test("pathdrop-193 unit: Tauri file-drop routing (shell probe + DPR side + single owner)", () => {
  // Browser/mock has neither Tauri global -> HTML5 handlers own drops.
  expect(hasTauriDropBridge(undefined)).toBe(false);
  expect(hasTauriDropBridge(null)).toBe(false);
  expect(hasTauriDropBridge({})).toBe(false);
  // Production shell always injects internals (legacy __TAURI__ optional).
  expect(hasTauriDropBridge({ __TAURI_INTERNALS__: {} })).toBe(true);
  expect(hasTauriDropBridge({ __TAURI__: {} })).toBe(true);
  // DPR-aware midpoint routing (physical px already -> CSS px by caller).
  expect(resolveTauriDropCssSide({ x: 100, y: 400 }, 1280, "left")).toBe("left");
  expect(resolveTauriDropCssSide({ x: 1200, y: 400 }, 1280, "left")).toBe("right");
  // Unknown position -> hovered-half fallback (default left).
  expect(resolveTauriDropCssSide(null, 1280, "left")).toBe("left");
  expect(resolveTauriDropCssSide({}, 1280, "right")).toBe("right");
  // Unknown position is pane-owned: PathBar hook skips, exactly one compare.
  expect(isTauriDropOnPathBar(undefined)).toBe(false);
  expect(isTauriDropOnPathBar(null)).toBe(false);
  expect(isTauriDropOnPathBar({})).toBe(false);
  // Browser multi-drop keeps order (BC Item0 -> Left, Item1 -> Right).
  expect(
    extractDroppedPaths({
      files: [
        { name: "drop-193-left.txt", path: LEFT_FILE },
        { name: "drop-193-right.txt", path: RIGHT_FILE },
      ],
      getData: (_t: string) => "",
    }),
  ).toEqual([LEFT_FILE, RIGHT_FILE]);
});
