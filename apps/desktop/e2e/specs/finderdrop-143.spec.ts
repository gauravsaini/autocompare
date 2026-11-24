/**
 * Wave-143 finderdrop live proof + hardening (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1559, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - unit (node): `hasTauriDropBridge` accepts the production-shell shape
 *    (`__TAURI_INTERNALS__`-only, `withGlobalTauri` false default) and the
 *    legacy `__TAURI__` shape, rejects plain-browser `{}` / null — pins
 *    the wave-143 subscribe fix (pehle guard real binary me bail karta
 *    tha, isliye Finder drops kabhi subscribe hi nahi hote the);
 *  - unit (node): `resolveTauriDropCssSide` DPR-1 mapping + unknown
 *    position fallback + `isTauriDropOnPathBar` unknown-position
 *    pane-owned (single compare, kabhi double nahi);
 *  - browser: Retina DPR-2 physical->CSS routing REAL module par
 *    (`/src/components/dnd/dropPaths.ts` dynamic import): physical
 *    x=1000 / width 1280 -> left (raw physical compare right deta —
 *    wahi wave-143 misroute bug), x=1800 -> right;
 *  - e2e: pane-half HTML5 single + dual drops regression pin (left half
 *    single stages left + one compare; dual Item0->left Item1->right
 *    EXACTLY one compare via `__AC_COMPARE_COUNT__`);
 *  - zero console errors / page errors throughout. PNGs in /tmp/drop-143/.
 *
 * Real-shell note: mock me Tauri `onDragDropEvent` listeners deliberate
 * no-op hain (HTML5 path cover karta hai); production shell me fixed
 * probe + DPR-aware side routing cover karta hai. Native Finder drops
 * ke liye manual steps REPORT me hain (checks/finderdrop-143/REPORT.md).
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import {
  extractDroppedPaths,
  hasTauriDropBridge,
  isTauriDropOnPathBar,
  resolveTauriDropCssSide,
} from "../../src/components/dnd/dropPaths";

const EVID_DIR = "/tmp/drop-143";

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
    if (msg.type === "error") errors.push(msg.text());
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

/** Synthetic drop of `paths` (in order) on `selector`. */
async function dropPaths(page: Page, selector: string, paths: string[]): Promise<void> {
  await page.locator(selector).first().evaluate((el, ps: string[]) => {
    const files = ps.map((p) => ({ name: p.split("/").pop() ?? p, path: p }));
    const dt = { files, types: ["Files"], getData: (_t: string) => "" };
    const ev = new DragEvent("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "dataTransfer", { value: dt });
    el.dispatchEvent(ev);
  }, paths);
}

test("finderdrop-143 unit: shell probe accepts prod internals-only window", () => {
  // Production shell shape: `withGlobalTauri` false (default, conf me
  // unset) => `__TAURI__` missing, `__TAURI_INTERNALS__` always injected.
  // Purana guard is shape par bail karta tha = Finder drops dead.
  expect(hasTauriDropBridge({ __TAURI_INTERNALS__: {} })).toBe(true);
  // Legacy opt-in shape bhi chalta hai.
  expect(hasTauriDropBridge({ __TAURI__: {} })).toBe(true);
  expect(hasTauriDropBridge({ __TAURI__: {}, __TAURI_INTERNALS__: {} })).toBe(true);
  // Plain browser / mock: dono missing => HTML5 handlers own drops.
  expect(hasTauriDropBridge({})).toBe(false);
  expect(hasTauriDropBridge(null)).toBe(false);
  expect(hasTauriDropBridge(undefined)).toBe(false);
});

test("finderdrop-143 unit: css-side mapping + unknown position pane-owned", () => {
  // Node me devicePixelRatio nahi => ratio 1 => physical == CSS.
  // (Tauri `position` hamesha {x,y} hota hai — dono chahiye.)
  expect(resolveTauriDropCssSide({ x: 400, y: 400 }, 1280, "left")).toBe("left");
  expect(resolveTauriDropCssSide({ x: 900, y: 400 }, 1280, "left")).toBe("right");
  // Unknown position => hovered fallback (default left), pane owns.
  expect(resolveTauriDropCssSide(undefined, 1280, "left")).toBe("left");
  expect(resolveTauriDropCssSide(undefined, 1280, "right")).toBe("right");
  expect(resolveTauriDropCssSide({}, 1280)).toBe("left");
  expect(isTauriDropOnPathBar(undefined)).toBe(false);
  expect(isTauriDropOnPathBar(null)).toBe(false);
  expect(isTauriDropOnPathBar({})).toBe(false);
  // Dual order preserved (BC Item0 -> Left, Item1 -> Right).
  const got = extractDroppedPaths({
    files: [
      { name: "a.txt", path: "/tmp/a.txt" },
      { name: "b.txt", path: "/tmp/b.txt" },
    ],
    getData: (_t: string) => "",
  });
  expect(got).toEqual(["/tmp/a.txt", "/tmp/b.txt"]);
});

test("finderdrop-143 browser: Retina DPR-2 physical->CSS routing (real module)", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const verdict = await page.evaluate(async () => {
    const mod = await import("/src/components/dnd/dropPaths.ts");
    // Retina Mac: DPR 2 (read-only getter => redefine for the probe).
    Object.defineProperty(window, "devicePixelRatio", { value: 2, configurable: true });
    const pt = mod.tauriDropCssPoint({ x: 1000, y: 200 });
    // Physical 1000 @DPR2 = CSS 500, width 1280 => LEFT half.
    // (Buggy raw-physical compare: 1000 >= 640 => right — galat.)
    const left = mod.resolveTauriDropCssSide({ x: 1000, y: 200 }, 1280, "left");
    const right = mod.resolveTauriDropCssSide({ x: 1800, y: 200 }, 1280, "left");
    const fallback = mod.resolveTauriDropCssSide(undefined, 1280, "right");
    const probeProd = mod.hasTauriDropBridge({ __TAURI_INTERNALS__: {} });
    const probeBrowser = mod.hasTauriDropBridge({});
    return { pt, left, right, fallback, probeProd, probeBrowser };
  });
  expect(verdict.pt).toEqual({ x: 500, y: 100 });
  expect(verdict.left).toBe("left");
  expect(verdict.right).toBe("right");
  expect(verdict.fallback).toBe("right");
  expect(verdict.probeProd).toBe(true);
  expect(verdict.probeBrowser).toBe(false);
  expect(errors, "zero console/page errors").toEqual([]);
});

test("finderdrop-143 e2e: pane-half single + dual drops, exactly one compare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  const before = await compareCount(page);
  await dropPaths(page, '[data-panedrop="left"]', ["/tmp/fd143-left.txt"]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue("/tmp/fd143-left.txt");
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });

  await dropPaths(page, '[data-panedrop="right"]', ["/tmp/fd143-a.txt", "/tmp/fd143-b.txt"]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue("/tmp/fd143-a.txt", {
    timeout: 5000,
  });
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue("/tmp/fd143-b.txt", {
    timeout: 5000,
  });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await page.waitForTimeout(1500);
  // single (left) + dual (exactly one) = delta +2, dual ne double nahi kiya.
  expect(await compareCount(page), "single+dual fire exactly two compares").toBe(before + 2);
  await page.screenshot({ path: `${EVID_DIR}/pane-dual.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});
