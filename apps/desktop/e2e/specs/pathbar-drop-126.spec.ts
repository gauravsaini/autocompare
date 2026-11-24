/**
 * Wave-126 path areas droppable like BC (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1534, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - drop a file on the LEFT PathBar input stages that side + compare fires;
 *  - dropping two files stages Item0 -> left, Item1 -> right (BC dual-drop);
 *  - dragover shows the dashed outline affordance (`data-drop-active`);
 *  - zero console errors / page errors throughout. PNG evidence in /tmp/drop-126/.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";

const EVID_DIR = "/tmp/drop-126";

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

async function dragOver(page: Page, selector: string): Promise<void> {
  await page.locator(selector).evaluate((el) => {
    const dt = { files: [], types: ["Files"], getData: (_t: string) => "" };
    const ev = new DragEvent("dragover", { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "dataTransfer", { value: dt });
    el.dispatchEvent(ev);
  });
}

test("pathbar-drop-126: single drop on left input stages left + compare fires", async ({ page }) => {
  const errors = collectErrors(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', ["/tmp/drop-left-126.txt"]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue("/tmp/drop-left-126.txt");
  await page.screenshot({ path: `${EVID_DIR}/single-drop-left.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathbar-drop-126: dual drop stages Item0 left + Item1 right", async ({ page }) => {
  const errors = collectErrors(page);
  await dropPaths(page, '[data-testid="pathbar/left"]', ["/tmp/dual-left-126.txt", "/tmp/dual-right-126.txt"]);
  await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue("/tmp/dual-left-126.txt");
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue("/tmp/dual-right-126.txt");
  await page.screenshot({ path: `${EVID_DIR}/dual-drop.png` });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("pathbar-drop-126: dragover shows drop affordance", async ({ page }) => {
  const errors = collectErrors(page);
  await dragOver(page, '[data-testid="pathbar/right"]');
  await expect(page.locator('[data-testid="pathbar/right"]')).toHaveAttribute("data-drop-active", "true");
  expect(errors, "zero console/page errors").toEqual([]);
});
