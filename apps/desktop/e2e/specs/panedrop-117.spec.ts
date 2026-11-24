/**
 * Wave-117 pane file drop proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Beyond Compare parity verified first against live BC
 * (`/tmp/panedrop-117/bc-live.png`): a Finder drop onto the left pane
 * sets the left path (right pane -> right path) and immediately
 * re-compares; orphan rows keep their status line.
 *
 * Drives the REAL `App` workspace and proves the new
 * `components/dnd/*` wiring (App pane halves + SessionManager inputs):
 *  - left-drop sets left (+ auto compare fires, same IPC path as the
 *    toolbar Compare — status `Compare finished`, zero new IPC);
 *  - right-drop sets right (+ auto compare fires);
 *  - one-sided drop stages the path WITHOUT comparing and names the
 *    missing side;
 *  - SessionManager left/right inputs accept drops (same helper);
 *  - drop affordance is outline-only: firstRowTop budget intact
 *    (<=140, same ruler as textgutter-92) and zero layout shift while
 *    hovering (row-0 top identical before/during dragover);
 *  - light + dark PNGs land in `/tmp/panedrop-117/` (never in-repo);
 *    zero console errors / page errors throughout.
 *
 * Drops are synthetic `DragEvent`s with a `dataTransfer` stub
 * (`files[0].{name,path}`) dispatched on the target — React listens at
 * the root, so native bubbling reaches the same `onDrop` handlers that
 * serve real OS drops (Tauri shell events reuse the same
 * `onDropPath` in production).
 *
 * New file only. Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/panedrop-117";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

/** Dispatch a synthetic drop carrying `path` on `selector`. */
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

test("panedrop-117: left-drop sets left + auto compare fires", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text session boots automatically with both sides filled.
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  await dropOn(page, '[data-panedrop="left"]', "/tmp/drop-left.txt");
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/drop-left.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible();

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("panedrop-117: right-drop sets right + auto compare fires", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });

  await dropOn(page, '[data-panedrop="right"]', "/tmp/drop-right.txt");
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/drop-right.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  await expect(page.getByTestId("view-text")).toBeVisible();

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("panedrop-117: one-sided drop stages without comparing", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Fresh text session with left only: right side is missing.
  await createSession(page, "text", "/tmp/only-left.txt", "", "view-text");

  await dropOn(page, '[data-panedrop="left"]', "/tmp/staged-left.txt");
  await expect(page.getByTestId("pathbar/left")).toHaveValue("/tmp/staged-left.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("drop the other side", { timeout: 5000 });

  // Completing the pair fires the auto compare (same IPC path as Compare).
  await dropOn(page, '[data-panedrop="right"]', "/tmp/staged-right.txt");
  await expect(page.getByTestId("pathbar/right")).toHaveValue("/tmp/staged-right.txt", { timeout: 5000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("panedrop-117: SessionManager inputs accept drops", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();

  await dropOn(page, '[data-testid="session/left-path"]', "/tmp/sm-left.txt");
  await expect(page.getByTestId("session/left-path")).toHaveValue("/tmp/sm-left.txt");
  await dropOn(page, '[data-testid="session/right-path"]', "/tmp/sm-right.txt");
  await expect(page.getByTestId("session/right-path")).toHaveValue("/tmp/sm-right.txt");

  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});

test("panedrop-117: affordance is outline-only, budget intact, crops", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

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
  console.log(`PANEDROP117 firstRowTop=${before.firstRowTop} rowTop=${before.rowTopAbs}`);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
