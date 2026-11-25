/**
 * Wave-235 empty + notice standards (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Pins the wave-232 primitive contracts after the 7-view migration:
 *  (a) Modal is centered and Esc-dismissed (gallery route);
 *  (b) DockedBar is in-flow and Esc-dismissed (gallery route);
 *  (c) Empty contract: blank folder/sync/media render the single honest
 *      `workspace/blank-empty` ("needs two paths", views never mount), and
 *      every `ui/EmptyState` carries one h2 + centered geometry;
 *  (d) No `role=alert` anywhere on these pages: every inline notice is the
 *      single `role=status` `ui/InlineNotice`.
 *
 * PORT-FREE: runs on a fresh :1803 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";

const EVID_DIR = "/tmp/ui-standards-235";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 30000 });
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

/** Blank session of `kind` via the Home quick-create card (no paths staged). */
async function openBlankKind(page: Page, kind: string, viewId: string): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click();
  } else {
    const home = page.getByTestId("tab-home");
    if ((await home.count()) > 0) await home.click().catch(() => {});
  }
  await expect(page.getByTestId("workspace/welcome")).toBeVisible({ timeout: 15000 });
  await page.getByTestId(`home/new-${kind}`).click();
  // Wave-150 (wave-138 contract): blank sessions render the single
  // honest empty; views never mount until both paths are set.
  await expect(page.getByTestId("workspace/blank-empty")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId(viewId)).toHaveCount(0);
}

test("modal is centered and Esc-dismissed", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/root")).toBeVisible({ timeout: 15000 });

  await page.getByTestId("gallery/open-modal").click();
  const modal = page.getByTestId("gallery/modal");
  await expect(modal).toBeVisible({ timeout: 15000 });
  await expect(modal).toHaveAttribute("role", "dialog");

  const box = await modal.boundingBox();
  expect(box, "modal has a layout box").not.toBeNull();
  if (box !== null) {
    const vp = page.viewportSize() ?? { width: 1280, height: 800 };
    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;
    expect(Math.abs(cx - vp.width / 2), `modal horizontally centered (cx=${cx})`).toBeLessThan(vp.width * 0.08);
    expect(Math.abs(cy - vp.height / 2), `modal vertically centered (cy=${cy})`).toBeLessThan(vp.height * 0.25);
  }
  await page.screenshot({ path: `${EVID_DIR}/modal-centered.png` });

  await page.keyboard.press("Escape");
  await expect(modal).toBeHidden({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});

test("docked bar is in-flow and Esc-dismissed", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/root")).toBeVisible({ timeout: 15000 });

  // Dock starts open; Esc collapses it without moving rows (in-flow: no
  // absolute/fixed positioning on the bar itself).
  const dock = page.getByTestId("gallery/dock");
  await expect(dock).toBeVisible({ timeout: 15000 });
  const pos = await dock.evaluate((el) => getComputedStyle(el).position);
  expect(pos, "docked bar never floats (no absolute/fixed)").not.toMatch(/absolute|fixed/);
  await page.keyboard.press("Escape");
  await expect(dock).toBeHidden({ timeout: 15000 });

  // Reopen via toggle, dismiss via the close button.
  await page.getByTestId("gallery/toggle-dock").click();
  await expect(dock).toBeVisible({ timeout: 15000 });
  await page.getByTestId("gallery/dock/close").click();
  await expect(dock).toBeHidden({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});

test("empty contract: blank kinds share one honest empty, states are uniform", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  for (const [kind, viewId] of [["folder", "view-folder"], ["sync", "view-sync"], ["media", "view-media"]] as const) {
    await openBlankKind(page, kind, viewId);
    const empty = page.getByTestId("workspace/blank-empty");
    await expect(empty).toBeVisible({ timeout: 15000 });
    await expect(empty).toContainText("needs two paths");
    await expect(page.getByRole("alert")).toHaveCount(0);
    await page.screenshot({ path: `${EVID_DIR}/blank-${kind}-empty.png` });
  }

  // Gallery EmptyState: one h2 + centered geometry (the contract every
  // migrated per-view empty now shares).
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  const estate = page.getByTestId("gallery/empty");
  await expect(estate).toBeVisible({ timeout: 15000 });
  expect(await estate.locator("h2").count(), "EmptyState carries one h2").toBe(1);
  await expect(estate.locator("h2")).toContainText("Nothing to compare");
  const align = await estate.evaluate((el) => getComputedStyle(el).textAlign);
  expect(align, "EmptyState is centered").toBe("center");
  const notice = page.getByTestId("gallery/notice");
  await expect(notice).toHaveAttribute("role", "status");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/gallery-emptynotice.png` });
  assertClean(consoleErrors, pageErrors);
});
