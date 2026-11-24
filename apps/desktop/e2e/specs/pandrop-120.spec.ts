/**
 * Wave-120 pane drop follow-ups proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Beyond Compare parity verified first against the observer matrix
 * (`/tmp/bc-drop-observe.md` sections 4-6) plus live-BC `Info.plist`
 * (`*`/`fold`/`disk`/`****` doc types; `NSServices` folder entries accept
 * `public.directory` + `public.archive`):
 *  - dual-file drop: Item 0 -> Left, Item 1 -> Right in ONE drop,
 *    regardless of half (observer section 3, `Count >= 2` loop);
 *  - folder-in-text auto-switch: folders/archives in a file compare open
 *    a Folder Compare (`BCVIEWS DIRVIEWNEW` idiom, observer section 4);
 *  - file-in-folder auto-switch: files in a folder compare open a
 *    Text compare (`BCVIEWS FILEVIEWNEW` idiom);
 *  - archives (`.zip`/`.tar`/`.gz`) count as folders
 *    (`SPECSAREARCHIVES`, observer sections 4+6).
 *
 * Drives the REAL `App` workspace and proves the extended
 * `components/dnd/*` wiring (117 `onDropPath` intact + new
 * `onDropPaths` dual path):
 *  - dual drop on EITHER half sets left=Item0 + right=Item1 + auto
 *    compare fires (same IPC path as the toolbar Compare, zero new IPC);
 *  - folder dropped in a text session opens a folder session
 *    (`view-folder`, dropped side set, other side carried only when
 *    already folderish, else `drop the other side`);
 *  - archive dropped in a text session opens a folder session too;
 *  - file dropped in a folder session opens a text session;
 *  - single-file same-side drop keeps exact 117 behavior (pin);
 *  - outline-only affordance: chrome budget intact (firstRowTop<=140)
 *    and zero layout shift while hovering;
 *  - light + dark PNGs land in `/tmp/pandrop-120/` (never in-repo);
 *    zero console errors / page errors throughout.
 *
 * Drops are synthetic `DragEvent`s with a `dataTransfer` stub
 * (`files[i].{name,path}`) dispatched on the target — React listens at
 * the root, so native bubbling reaches the same `onDrop` handlers that
 * serve real OS drops (Tauri shell events reuse the same
 * `onDropPath(s)` in production).
 *
 * New file only. Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/pandrop-120";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

/** Dispatch a synthetic single-path drop carrying `path` on `selector`. */
async function dropOn(page: Page, selector: string, path: string): Promise<void> {
  await page.locator(selector).evaluate(
    (el, p) => {
      const name = p.split("/").pop() ?? p;
      const dt = {
        files: [{ name, path: p }],
        types: ["Files"],
        getData: (_type: string) => "",
      };
      const ev = new DragEvent("drop", { bubbles: true, cancelable: true });
      Object.defineProperty(ev, "dataTransfer", { value: dt });
      el.dispatchEvent(ev);
    },
    path,
  );
}

/** Dispatch a synthetic multi-path drop (ONE event, N files) on `selector`. */
async function dropManyOn(page: Page, selector: string, paths: readonly string[]): Promise<void> {
  await page.locator(selector).evaluate(
    (el, ps) => {
      const files = (ps as unknown as string[]).map((p) => ({
        name: p.split("/").pop() ?? p,
        path: p,
      }));
      const dt = {
        files,
        types: ["Files"],
        getData: (_type: string) => "",
      };
      const ev = new DragEvent("drop", { bubbles: true, cancelable: true });
      Object.defineProperty(ev, "dataTransfer", { value: dt });
      el.dispatchEvent(ev);
    },
    paths as unknown as string,
  );
}

/** Dispatch a synthetic dragover on the `side` half (holds the affordance). */
async function dragOver(page: Page, side: "left" | "right"): Promise<void> {
  await page.locator(`[data-panedrop="${side}"]`).evaluate((el) => {
    const dt = { files: [], types: ["Files"], getData: (_type: string) => "" };
    const ev = new DragEvent("dragover", { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "dataTransfer", { value: dt });
    Object.defineProperty(ev, "clientX", { value: 0 });
    el.dispatchEvent(ev);
  });
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
  if (left !== "") await page.getByTestId("session/left-path").fill(left);
  if (right !== "") await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewId)).toBeVisible();
}

function errorWatches(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function assertClean(consoleErrors: string[], pageErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

test("pandrop-120: dual-file drop sets left+right ek saath + auto compare", async ({ page }) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  // Demo text session boots automatically with both sides filled.
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // ONE drop of two files on the LEFT half: Item0 -> Left, Item1 -> Right.
  await dropManyOn(page, '[data-panedrop="left"]', ["/tmp/dual-left.txt", "/tmp/dual-right.txt"]);
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/dual-left.txt", { timeout: 5000 });
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/dual-right.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await expect(page.getByTestId("view-text")).toBeVisible();

  // Same pair routed by the RIGHT half too (BC: side-independent).
  await dropManyOn(page, '[data-panedrop="right"]', ["/tmp/dual2-left.txt", "/tmp/dual2-right.txt"]);
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/dual2-left.txt", { timeout: 5000 });
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/dual2-right.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });

  assertClean(consoleErrors, pageErrors);
});

test("pandrop-120: folder dropped in text session auto-switches to folder compare", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // BC DIRVIEWNEW idiom: folder in a file compare opens Folder Compare.
  await dropOn(page, '[data-panedrop="left"]', "/tmp/folderA/");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/folderA/", { timeout: 5000 });
  // Demo right side was a file: not carried, so the stage note names it.
  await expect(page.getByTestId("statusbar/message")).toContainText("opened folder compare", {
    timeout: 5000,
  });

  assertClean(consoleErrors, pageErrors);
});

test("pandrop-120: archive dropped in text session counts as folder", async ({ page }) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // BC SPECSAREARCHIVES idiom: `.zip` behaves like a folder drop.
  await dropOn(page, '[data-panedrop="right"]', "/tmp/bundle.zip");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/bundle.zip", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("opened folder compare", {
    timeout: 5000,
  });

  assertClean(consoleErrors, pageErrors);
});

test("pandrop-120: file dropped in folder session auto-switches to text compare", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  // Fresh folder session with both sides folderish.
  await createSession(page, "folder", "/tmp/fA/", "/tmp/fB/", "view-folder");

  // BC FILEVIEWNEW idiom: file in a folder compare opens Text Compare.
  await dropOn(page, '[data-panedrop="left"]', "/tmp/a.txt");
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/a.txt", { timeout: 5000 });
  // Folder right side is not carried into a text session: stage note.
  await expect(page.getByTestId("statusbar/message")).toContainText("opened text compare", {
    timeout: 5000,
  });

  assertClean(consoleErrors, pageErrors);
});

test("pandrop-120: dual folders in text session switch ek saath", async ({ page }) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  // Two folders in ONE drop: Folder Compare opens with BOTH sides set.
  await dropManyOn(page, '[data-panedrop="left"]', ["/tmp/fA/", "/tmp/fB/"]);
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/fA/", { timeout: 5000 });
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/fB/", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });

  assertClean(consoleErrors, pageErrors);
});

test("pandrop-120: single-file same-side drop unchanged (117 pin)", async ({ page }) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  await dropOn(page, '[data-panedrop="left"]', "/tmp/drop-left.txt");
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/drop-left.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible();

  assertClean(consoleErrors, pageErrors);
});

test("pandrop-120: affordance is outline-only, budget intact, crops", async ({ page }) => {
  const { consoleErrors, pageErrors } = errorWatches(page);

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  const metrics = () =>
    page.evaluate(() => {
      const q = (s: string): Element | null => document.querySelector(s);
      const r = (el: Element | null): DOMRect | null =>
        el instanceof HTMLElement ? el.getBoundingClientRect() : null;
      const section = q('section[aria-label="text comparison"]');
      const first = q('[data-testid="textview/row-0"]');
      const sr = r(section);
      const fr = r(first);
      return {
        firstRowTop: sr && fr ? Math.round(fr.top - sr.top) : null,
        rowTopAbs: fr ? Math.round(fr.top) : null,
      };
    });

  const before = await metrics();
  expect(before.firstRowTop, "chrome budget firstRowTop<=140").not.toBeNull();
  expect(before.firstRowTop as number).toBeLessThanOrEqual(140);

  // Hover the left half: affordance arms, layout does NOT move.
  await dragOver(page, "left");
  await expect(page.locator('[data-panedrop="left"]')).toHaveAttribute("data-active", "true");
  const during = await metrics();
  expect(during.rowTopAbs, "zero layout shift during dragover").toBe(before.rowTopAbs);
  expect(during.firstRowTop, "budget intact during dragover").toBeLessThanOrEqual(140);
  await page.locator('[data-panedrop="left"]').screenshot({ path: `${EVID_DIR}/afford-left-light.png` });

  await dragOver(page, "right");
  await expect(page.locator('[data-panedrop="right"]')).toHaveAttribute("data-active", "true");
  await page.locator('[data-panedrop="right"]').screenshot({ path: `${EVID_DIR}/afford-right-light.png` });

  await page.screenshot({ path: `${EVID_DIR}/panes-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await dragOver(page, "left");
  await expect(page.locator('[data-panedrop="left"]')).toHaveAttribute("data-active", "true");
  await page.locator('[data-panedrop="left"]').screenshot({ path: `${EVID_DIR}/afford-left-dark.png` });
  await page.screenshot({ path: `${EVID_DIR}/panes-dark.png` });
  console.log(`PANDROP120 firstRowTop=${before.firstRowTop} rowTop=${before.rowTopAbs}`);

  assertClean(consoleErrors, pageErrors);
});
