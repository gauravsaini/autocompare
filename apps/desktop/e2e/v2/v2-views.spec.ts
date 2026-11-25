/**
 * AutoCompare v2 — All Native Views living E2E spec.
 *
 * Covers the 7 non-text native v2 comparison views:
 * Folder, Table, Hex, 3-Way Text Merge, 3-Way Folder Merge, Sync, and Media.
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/?v2=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("v2/shell")).toBeVisible({ timeout: 15000 });
});

const DUAL_PATH_VIEWS: ReadonlyArray<readonly [string, string]> = [
  ["folder", "v2/folder-view"],
  ["table", "v2/table-view"],
  ["hex", "v2/hex-view"],
  ["sync", "v2/sync-view"],
  ["media", "v2/media-view"],
];

for (const [kind, viewTestId] of DUAL_PATH_VIEWS) {
  test(`dual-pane view (${kind}) renders V2DualPathBar and zero errors`, async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(String(e)));

    await page.getByTestId(`v2/home-card-${kind}`).click();
    await expect(page.getByTestId(viewTestId)).toBeVisible({ timeout: 10000 });
    await expect(page.getByTestId("v2/path-left")).toBeVisible();
    await expect(page.getByTestId("v2/path-right")).toBeVisible();
    await expect(page.getByTestId("v2/statusbar")).toBeVisible();
    await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
    expect(errors).toEqual([]);
  });
}

const THREE_WAY_VIEWS: ReadonlyArray<readonly [string, string]> = [
  ["merge", "v2/merge-view"],
  ["folderMerge", "v2/foldermerge-view"],
];

for (const [kind, viewTestId] of THREE_WAY_VIEWS) {
  test(`3-way view (${kind}) renders cleanly with zero errors`, async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(String(e)));

    await page.getByTestId(`v2/home-card-${kind}`).click();
    await expect(page.getByTestId(viewTestId)).toBeVisible({ timeout: 10000 });
    await expect(page.getByTestId("v2/statusbar")).toBeVisible();
    await expect(page.getByTestId("v2/error-banner")).toHaveCount(0);
    expect(errors).toEqual([]);
  });
}
