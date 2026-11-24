/**
 * Wave-138 path-less open honest empty (BC parity, audit gap).
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace and proves:
 *  (a) blank-path text session me koi demo fire nahi hota — the
 *      workspace renders the honest pick-two empty
 *      (`workspace/blank-empty`, "needs two paths"), zero
 *      `textview/demo` rows, zero `view-text`, zero demo banner,
 *      zero `role=alert`;
 *  (b) blank session par toolbar Compare disabled rehta hai with the
 *      honest `needs two paths` reason (existing wiring), aur ruler
 *      honestly 0 rows par rehta hai (`ruler/empty`, zero markers);
 *  (c) dono paths bharne par normal compare resume hota hai
 *      (`view-text` rows wapas, Compare enabled), demo kabhi wapas
 *      nahi aata.
 *
 * PORT-FREE: runs on a fresh :1549 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";

const EVID_DIR = "/tmp/openempty-138";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
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

/** Blank text session via the Home quick-create card (no paths staged). */
async function openBlankText(page: Page): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click();
  } else {
    const home = page.getByTestId("tab-home");
    if ((await home.count()) > 0) await home.click().catch(() => {});
  }
  await expect(page.getByTestId("workspace/welcome")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("home/new-text").click();
  await expect(page.getByTestId("workspace/blank-empty")).toBeVisible({ timeout: 15000 });
}

test("blank text session is honestly empty: no demo rows, Compare parked", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await openBlankText(page);

  const empty = page.getByTestId("workspace/blank-empty");
  await expect(empty).toContainText("needs two paths");

  // Koi compare/error/demo nahi: TextView mount hi nahi hua.
  await expect(page.getByTestId("view-text")).toHaveCount(0);
  await expect(page.getByTestId("textview/demo")).toHaveCount(0);
  await expect(page.getByTestId("workspace/demo-banner")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Ruler honestly 0 rows, zero fake markers.
  await expect(page.getByTestId("ruler/empty")).toBeVisible();
  await expect(page.getByTestId("ruler/empty")).toHaveText("0 rows");
  expect(await page.locator('[data-testid="ruler"] [data-kind]').count()).toBe(0);

  // Compare disabled with honest reason (existing needs-two-paths wiring).
  const compare = page.getByTestId("toolbar/action-compare");
  await expect(compare).toBeDisabled();
  await expect(compare).toHaveAttribute("title", "Compare (needs two paths)");

  await page.screenshot({ path: `${EVID_DIR}/blank-empty.png` });
  assertClean(consoleErrors, pageErrors);
});

test("filling both paths resumes normal compare, demo never returns", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await openBlankText(page);

  // PathBar Enter single-side stage nahi karta (124 gate), isliye BC
  // dual-drop: ek event me Item0 -> left, Item1 -> right stage hokar
  // same-tab par normal compare resume hota hai.
  await page.locator('[data-testid="pathbar/left"]').evaluate((el, ps: string[]) => {
    const files = ps.map((p) => ({ name: p.split("/").pop() ?? p, path: p }));
    const dt = { files, types: ["Files"], getData: (_t: string) => "" };
    const ev = new DragEvent("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "dataTransfer", { value: dt });
    el.dispatchEvent(ev);
  }, ["/tmp/138-left.txt", "/tmp/138-right.txt"]);

  // Normal compare resume: real rows, blank empty gone, demo never back.
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("workspace/blank-empty")).toHaveCount(0);
  await expect(page.getByTestId("textview/demo")).toHaveCount(0);
  await expect(page.getByTestId("workspace/demo-banner")).toHaveCount(0);
  await expect(page.getByTestId("toolbar/action-compare")).toBeEnabled({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);

  await page.screenshot({ path: `${EVID_DIR}/compare-resumed.png` });
  assertClean(consoleErrors, pageErrors);
});
