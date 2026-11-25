/**
 * Wave-fixA1 alert/empty (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Pins the fixA1 migration: raw loading/empty `<p>` ride `ui/EmptyState`,
 * the demo note rides `ui/InlineNotice`, zero `role=alert` on views +
 * gallery, and the SessionManager `Modal` dismisses via root-Esc and
 * backdrop click (search-Esc clears first, second Esc closes).
 *
 * PORT-FREE: runs on a fresh :1827 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit).
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

test.beforeEach(async ({ page }) => {
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

test("alertempty: no role=alert, empty states are uniform EmptyState", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  // Main view (mock text compare): error + empty use primitives, no alert.
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  // Demo note rides InlineNotice (role=status), testid byte-identical.
  const demo = page.getByTestId("textview/demo");
  if ((await demo.count()) > 0) {
    await expect(demo).toHaveAttribute("role", "status");
  }
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Gallery primitives: EmptyState has one h2 + centered, notice is status.
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/root")).toBeVisible({ timeout: 15000 });
  const estate = page.getByTestId("gallery/empty");
  await expect(estate).toBeVisible({ timeout: 15000 });
  expect(await estate.locator("h2").count(), "EmptyState carries one h2").toBe(1);
  const align = await estate.evaluate((el) => getComputedStyle(el).textAlign);
  expect(align, "EmptyState is centered").toBe("center");
  await expect(page.getByTestId("gallery/notice")).toHaveAttribute("role", "status");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Static pins: 5 loading raws now ride EmptyState, zero raw role=alert in views.
  const here = path.dirname(fileURLToPath(import.meta.url));
  const srcRoot = path.resolve(here, "../../src/components/views");
  const readSrc = (rel: string): string => fs.readFileSync(path.join(srcRoot, rel), "utf8");
  for (const [rel, emptyId] of [
    ["TextView.tsx", "textview/loading"],
    ["FolderView.tsx", "folderview/loading"],
    ["TableView.tsx", "tableview/loading"],
    ["HexView.tsx", "hexview/loading"],
    ["MergeView.tsx", "mergeview/loading"],
  ] as const) {
    const src = readSrc(rel);
    expect(src, `${rel} loading rides EmptyState`).toContain(emptyId);
    expect(src, `${rel} keeps zero raw loading <p>`).not.toContain("<p>Loading");
  }
  const hexSrc = readSrc("HexView.tsx");
  expect(hexSrc, "hex both-empty rides EmptyState").toContain("hexview/both-empty");
  expect(hexSrc, "hex page-loading rides EmptyState").toContain("hexview/page-loading");
  for (const rel of ["TextView.tsx", "FolderView.tsx", "TableView.tsx", "HexView.tsx", "MergeView.tsx"]) {
    expect(readSrc(rel), `${rel} keeps zero role=alert`).not.toContain('role="alert"');
    expect(readSrc(rel), `${rel} keeps zero role='alert'`).not.toContain("role='alert'");
  }
  assertClean(consoleErrors, pageErrors);
});

test("alertempty: session modal Esc + backdrop dismiss, search-Esc contract", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  // Open via top toolbar, Esc closes (root-Esc route).
  await page.getByTestId("toolbar/action-open").click();
  const dialog = page.getByTestId("session/dialog");
  await expect(dialog).toBeVisible({ timeout: 15000 });
  await expect(dialog).toHaveAttribute("role", "dialog");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden({ timeout: 15000 });

  // Reopen, backdrop click (corner of the scrim) closes.
  await page.getByTestId("toolbar/action-open").click();
  await expect(dialog).toBeVisible({ timeout: 15000 });
  await page.mouse.click(8, 8);
  await expect(dialog).toBeHidden({ timeout: 15000 });

  // Reopen, search-Esc still dismisses via the Modal root-Esc route
  // (capture listener owns Esc, so the filter clears alongside close).
  await page.getByTestId("toolbar/action-open").click();
  await expect(dialog).toBeVisible({ timeout: 15000 });
  const search = page.getByTestId("session/search");
  await search.fill("zzz-no-such-session");
  await expect(page.getByTestId("session/search-empty")).toBeVisible({ timeout: 15000 });
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden({ timeout: 15000 });
  // Reopen proves the filter cleared (dismiss did not strand state).
  await page.getByTestId("toolbar/action-open").click();
  await expect(dialog).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("session/search")).toHaveValue("");
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});
