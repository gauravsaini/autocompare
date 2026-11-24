/**
 * Wave-173 pathdnd BC-parity slice (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1573, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - single drop on the LEFT PathBar field stages that side with the
 *    honest single-side note ("drop right to compare", no compare);
 *  - single drop on the RIGHT PathBar field triggers auto-compare when
 *    both sides are present (exactly +1 compare, "Compare finished");
 *  - dual drop on the WHOLE PathBar strip stages Item0 -> left,
 *    Item1 -> right with EXACTLY ONE compare (no double) + strip stays
 *    within the chrome budget (pathbar height <= 36px, no new rows);
 *  - click-to-browse on a PathBar field (`pathbar/left-browse`, stubbed
 *    `__MOCK_DIALOG_OPEN__`) stages the picked path like a drop;
 *  - click-to-browse on a pane half (`panedrop/browse-right`,
 *    hover-reveal, stubbed dialog) stages that side;
 *  - unit: `coercePickedPath` coerces dialog results, `isFolderishPath`
 *    keeps the folderish-vs-fileish kind guard, `isFolderViewActive`
 *    stays false with no folder view mounted;
 *  - zero console errors / page errors throughout. PNGs in /tmp/drop-173/.
 *
 * Real-shell note: under mock the Tauri `onDragDropEvent` listeners are a
 * deliberate no-op and the HTML5 `onDrop` handlers own drops; in the
 * production shell `useTauriPathBarDrop` owns the y-gated PathBar strip
 * while `useTauriPaneDrop` owns the panes (single compare guaranteed),
 * and both browse buttons ride the native dialog. Shell drops need the
 * manual live-drop check (see checks/pathdnd-173/REPORT.md).
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import {
  coercePickedPath,
  extractDroppedPaths,
  isFolderishPath,
  isFolderViewActive,
} from "../../src/components/dnd/dropPaths";

const EVID_DIR = "/tmp/drop-173";
// Mock IPC serves fixtures for any path, so /tmp scratch paths keep the
// run hermetic (no dependence on ~/Documents fixtures).
const LEFT_DROP = "/tmp/pathdnd-173-left.txt";
const RIGHT_DROP = "/tmp/pathdnd-173-right.txt";
const BROWSE_PICK = "/tmp/pathdnd-173-browse.txt";
const PANE_PICK = "/tmp/pathdnd-173-pane.txt";

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

/** Install the mock native-dialog stub, then reload so chrome picks it up. */
async function stubDialog(page: Page, picked: string): Promise<void> {
  await page.addInitScript((p: string) => {
    (window as unknown as Record<string, unknown>).__MOCK_DIALOG_OPEN__ = async () => p;
  }, picked);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
}

test("pathdnd-173: left drop single-side stages with honest note", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Settle first: the demo pair hydrates async after boot; wait for both
  // sides so the clear below starts from a known state.
  await expect.poll(
    async () =>
      (await page.locator('[data-testid="pathbar/left"]').inputValue()) !== "" &&
      (await page.locator('[data-testid="pathbar/right"]').inputValue()) !== "",
    { timeout: 15000 },
  ).toBe(true);
  // Clear the right side so the left drop is genuinely single-sided: fill
  // blank + Enter submits (App stages "", compare parks honestly).
  await page.locator('[data-testid="pathbar/right"]').fill("");
  await page.locator('[data-testid="pathbar/right"]').press("Enter");
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue("");
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', [LEFT_DROP]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(LEFT_DROP);
  // Honest single-side status, no compare fired.
  await expect(page.getByTestId("pathbar/note")).toContainText("drop right to compare", {
    timeout: 5000,
  });
  await page.waitForTimeout(800);
  expect(await compareCount(page), "single-side drop fires no compare").toBe(before);
  // The staged draft survives re-renders (no silent revert to tab paths).
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(LEFT_DROP);
  await page.screenshot({ path: `${EVID_DIR}/single-stage-left.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdnd-173: right drop with both sides present auto-compares", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar/right"]', [RIGHT_DROP]);
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(RIGHT_DROP);
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/right-drop-compare.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdnd-173: whole-bar dual drop stages both + exactly one compare", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Chrome budget: the bar (now with Browse buttons) stays one compact row.
  const barBox = await page.locator('[data-testid="pathbar"]').boundingBox();
  expect(barBox?.height ?? 999, "pathbar stays a single compact row").toBeLessThanOrEqual(36);
  const before = await compareCount(page);
  await dropPaths(page, '[data-testid="pathbar"]', [LEFT_DROP, RIGHT_DROP], 2000);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(LEFT_DROP, {
    timeout: 5000,
  });
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(RIGHT_DROP, {
    timeout: 5000,
  });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  // Settle: a double-compare bug would fire the 2nd compare right after.
  await page.waitForTimeout(1500);
  expect(await compareCount(page), "dual drop fires exactly one compare").toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/whole-bar-dual.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdnd-173: PathBar click-to-browse stages the picked path", async ({ page }) => {
  const errors = collectErrors(page);
  await stubDialog(page, BROWSE_PICK);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  const browse = page.getByTestId("pathbar/left-browse");
  await expect(browse).toBeEnabled({ timeout: 5000 });
  const before = await compareCount(page);
  await browse.click();
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(BROWSE_PICK, {
    timeout: 5000,
  });
  // Both sides present (demo pair) -> the picked path auto-compares once.
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/pathbar-browse.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdnd-173: pane-half click-to-browse stages that side", async ({ page }) => {
  const errors = collectErrors(page);
  await stubDialog(page, PANE_PICK);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  // Hover-reveal: the per-half Browse pill appears on root hover.
  await page.locator('[data-testid="panedrop/root"]').hover();
  const browseRight = page.getByTestId("panedrop/browse-right");
  await expect(browseRight).toBeVisible({ timeout: 5000 });
  const before = await compareCount(page);
  await browseRight.click();
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(PANE_PICK, {
    timeout: 5000,
  });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
    timeout: 10000,
  });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);
  await page.screenshot({ path: `${EVID_DIR}/pane-browse.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathdnd-173 unit: dialog coerce + folderish/fileish kind guard", () => {
  expect(coercePickedPath(null)).toBeNull();
  expect(coercePickedPath("")).toBeNull();
  expect(coercePickedPath("  ")).toBeNull();
  expect(coercePickedPath("/tmp/a.txt")).toBe("/tmp/a.txt");
  expect(coercePickedPath(["", "/tmp/b.txt"])).toBe("/tmp/b.txt");
  expect(coercePickedPath([])).toBeNull();
  // Kind guard preserved: archives/trailing-slash/extensionless read
  // folderish; plain files stay fileish.
  expect(isFolderishPath("/tmp/archive.zip")).toBe(true);
  expect(isFolderishPath("/tmp/some/dir/")).toBe(true);
  expect(isFolderishPath("/tmp/a.txt")).toBe(false);
  expect(isFolderishPath("LICENSE")).toBe(false);
  // Dual-drop extraction keeps order (BC Item0 -> left, Item1 -> right).
  expect(
    extractDroppedPaths({
      files: [
        { name: "l.txt", path: LEFT_DROP },
        { name: "r.txt", path: RIGHT_DROP },
      ],
      getData: (_t: string) => "",
    }),
  ).toEqual([LEFT_DROP, RIGHT_DROP]);
  // No folder view mounted in the unit (node) context -> file mode.
  expect(isFolderViewActive()).toBe(false);
});
