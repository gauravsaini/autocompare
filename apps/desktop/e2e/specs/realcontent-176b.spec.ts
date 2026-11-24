/**
 * Wave-176b realcontent retry: Tauri BC-parity slice (mock renders REAL file rows).
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Ground truth (python difflib, CRLF-normalized like the Rust engine):
 * File 1 = 170 lines, File 2 = 159 lines -> 176 rows total:
 * 147 equal, 6 changed, 17 left-only, 6 right-only, 10 hunks.
 * (Wave-176 helper/spec carried wrong counts 16/8/5 from the raw
 * non-normalized probe; this spec pins the corrected values.)
 *
 *  - T1 real paths -> real rows + exactly 10 hunk gutter pairs.
 *  - T2 fake paths -> 100% back-compat fallback to dumped.json fixture
 *    (644 rows, 623 same, `common line 0000 ...`).
 *  - T3 row/hunk boundaries match difflib opcodes row-for-row (+-0).
 */
import { expect, test, type Page } from "@playwright/test";
import {
  GROUND_TRUTH_176,
  installRealContentMock,
  prepareRealContentFiles,
  type RealContentFixture,
} from "../helpers/realcontent";

function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(String(err)));
  return errors;
}

async function submitPaths(page: Page, left: string, right: string): Promise<void> {
  // Each Enter stages one side in App.tabPaths then auto-compares; the
  // second submit carries both new paths, so BOTH sides need an Enter.
  // (A single right-side Enter leaves left on the demo session path.)
  await page.getByTestId("pathbar/left").fill(left);
  await page.getByTestId("pathbar/left").press("Enter");
  await page.getByTestId("pathbar/right").fill(right);
  await page.getByTestId("pathbar/right").press("Enter");
}

test.describe("Wave-176b realcontent: mock renders REAL file rows", () => {
  let fixture: RealContentFixture;

  test.beforeAll(() => {
    fixture = prepareRealContentFiles();
  });

  test.afterAll(() => {
    if (fixture) fixture.cleanup();
  });

  test.beforeEach(async ({ page }) => {
    try {
      await page.goto("/", { waitUntil: "domcontentloaded" });
      await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    } catch (err) {
      console.warn("Browser navigation failed:", err);
      test.skip(true, `Browser unavailable: ${err}`);
    }
  });

  test("1. Real files -> real rows, 10 hunk gutter pairs, zero errors", async ({ page }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);
    await submitPaths(page, fixture.file1Path, fixture.file2Path);

    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    const status = page.getByTestId("textview/status");
    await expect(status).toContainText("176 / 176 rows", { timeout: 15000 });
    await expect(status).toContainText("147 same");
    await expect(status).toContainText("6 changed");
    await expect(status).toContainText("17 left-only");
    await expect(status).toContainText("6 right-only");
    await expect(status).toContainText("left 170 lines · right 159 lines");

    // Row 0 is a real deletion (File1 blank first line), NOT fixture text.
    const row0 = page.getByTestId("textview/row-0");
    await expect(row0).toBeVisible();
    await expect(row0).toHaveAttribute("data-kind", "leftOnly");
    await expect(row0).not.toContainText("common line 0000");
    // Row 1 carries real File1 content.
    await expect(page.getByTestId("textview/row-1")).toContainText(
      "com.android.hotwordenrollment.xgoogle",
    );

    // Exactly 10 hunk gutter pairs mount (ordinals 0..9), no 11th.
    for (let h = 0; h < GROUND_TRUTH_176.totalHunks; h += 1) {
      await expect(page.getByTestId(`hunk-copy-left-${h}`)).toBeVisible();
      await expect(page.getByTestId(`hunk-copy-right-${h}`)).toBeVisible();
    }
    await expect(page.getByTestId("hunk-copy-left-10")).toHaveCount(0);
    await expect(page.getByTestId("hunk-copy-right-10")).toHaveCount(0);

    expect(errors, "zero console/page errors").toEqual([]);
  });

  test("2. Fake paths -> dumped.json fixture fallback (back-compat)", async ({ page }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);
    await submitPaths(page, "/tmp/nonexistent-fake-left-176b.txt", "/tmp/nonexistent-fake-right-176b.txt");

    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    const status = page.getByTestId("textview/status");
    // Only the first 200-row window is loaded, so counts are partial
    // ("200 / 644 rows ... (loaded)"); the totals + row-0 prove fallback.
    await expect(status).toContainText("644 rows", { timeout: 15000 });
    await expect(status).toContainText("4 changed");

    await expect(page.getByTestId("textview/row-0")).toContainText(
      "common line 0000 with stable body text",
    );

    expect(errors, "zero console/page errors").toEqual([]);
  });

  test("3. Hunk boundaries match python difflib opcodes row-for-row", async ({ page }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);
    await submitPaths(page, fixture.file1Path, fixture.file2Path);

    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId("textview/status")).toContainText("176 / 176 rows", {
      timeout: 15000,
    });

    const kind = (i: number) => page.getByTestId(`textview/row-${i}`);
    // Hunk 0 delete(0..2), equal 3, hunk 1 delete(4..8), equal 9.
    await expect(kind(0)).toHaveAttribute("data-kind", "leftOnly");
    await expect(kind(2)).toHaveAttribute("data-kind", "leftOnly");
    await expect(kind(3)).toHaveAttribute("data-kind", "equal");
    await expect(kind(4)).toHaveAttribute("data-kind", "leftOnly");
    await expect(kind(8)).toHaveAttribute("data-kind", "leftOnly");
    await expect(kind(9)).toHaveAttribute("data-kind", "equal");
    // Hunk 2 insert(25..27), equal 28.
    await expect(kind(25)).toHaveAttribute("data-kind", "rightOnly");
    await expect(kind(27)).toHaveAttribute("data-kind", "rightOnly");
    await expect(kind(28)).toHaveAttribute("data-kind", "equal");
    // Hunk 3 replace(34..37: changed + leftOnly tail), equal 38.
    await expect(kind(34)).toHaveAttribute("data-kind", "changed");
    await expect(kind(37)).toHaveAttribute("data-kind", "leftOnly");
    await expect(kind(38)).toHaveAttribute("data-kind", "equal");
    // Hunk 5 single replace row 44, hunk 6 single insert row 56.
    await expect(kind(44)).toHaveAttribute("data-kind", "changed");
    await expect(kind(56)).toHaveAttribute("data-kind", "rightOnly");
    // Hunk 9 replace(157..161: changed + 4 leftOnly), equal tail to end.
    await expect(kind(157)).toHaveAttribute("data-kind", "changed");
    await expect(kind(161)).toHaveAttribute("data-kind", "leftOnly");
    await expect(kind(162)).toHaveAttribute("data-kind", "equal");
    await expect(kind(175)).toHaveAttribute("data-kind", "equal");

    expect(errors, "zero console/page errors").toEqual([]);
  });
});
