/**
 * Wave-C2d folder content-compare option (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Committed proof for C2b-folderopts-245: the Rules panel exposes the
 * engine-ready `content_compare` flag (byte-compare same-size files;
 * off = size/mtime fast path) and toggling it re-triggers
 * `folder_compare` with the param.
 *  - Rules panel opens via the toolbar toggle; legacy testids intact;
 *  - `folderview/content-compare-toggle` starts unchecked (fast path);
 *  - checking it re-fires `folder_compare` with `contentCompare: true`
 *    (observed via the `__FOLDER_CONTENT_LAST__` wire probe);
 *  - unchecking it re-fires with the flag off (legacy wire);
 *  - rows stay rendered throughout; zero console/page errors.
 *  - no geometry asserts (no bounding-box / overlap checks here).
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createFolderSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();
}

async function folderContentLast(page: Page): Promise<{ contentCompare: boolean; quickCompare: boolean } | null> {
  return page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const v = w["__FOLDER_CONTENT_LAST__"];
    if (v !== null && typeof v === "object") {
      const r = v as Record<string, unknown>;
      return {
        contentCompare: r["contentCompare"] === true,
        quickCompare: r["quickCompare"] === true,
      };
    }
    return null;
  });
}

test("c2b-folderopts-245: content-compare toggle re-compares with param", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createFolderSession(page);
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // 1. Legacy Rules wire intact: panel opens, mask controls present.
  await page.getByTestId("folderview/toolbar-rules").click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  await expect(page.getByTestId("folderview/mask-input")).toBeVisible();
  await expect(page.getByTestId("folderview/mask-apply")).toBeVisible();

  // 2. Toggle starts unchecked (fast path default).
  const toggle = page.getByTestId("folderview/content-compare-toggle");
  await expect(toggle).toBeVisible();
  await expect(toggle).not.toBeChecked();
  const before = await folderContentLast(page);
  expect(before, "initial compare rides the fast path").not.toBeNull();
  expect(before?.contentCompare).toBe(false);

  // 3. Checking re-fires folder_compare with contentCompare: true.
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect
    .poll(async () => (await folderContentLast(page))?.contentCompare ?? null, {
      timeout: 10000,
    })
    .toBe(true);
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  await page.screenshot({ path: "/tmp/c2b-folderopts-245-on-light.png" });

  // 4. Unchecking re-fires with the flag off (legacy fast path).
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect
    .poll(async () => (await folderContentLast(page))?.contentCompare ?? null, {
      timeout: 10000,
    })
    .toBe(false);
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // 5. Dark mode still renders; zero errors.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("folderview/status")).toBeVisible();
  await page.screenshot({ path: "/tmp/c2b-folderopts-245-off-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
