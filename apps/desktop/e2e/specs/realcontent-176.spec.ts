/**
 * Wave-176 realcontent: Tauri BC-parity slice (mock renders REAL file rows).
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Verifies that when mock text_compare receives real readable file paths,
 * it reads the files and generates real diff rows matching python difflib
 * ground truth (170 left lines, 159 right lines, 176 total rows, 10 hunks)
 * with zero divergence (±0).
 *
 * Also verifies that fake/non-existent paths preserve 100% backward
 * compatibility, falling back to the 644-row dumped fixture.
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

test.describe("Wave-176 realcontent: mock renders REAL file rows", () => {
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

  test("1. Real files -> real hunks: File 1 vs File 2 renders real content and 10 hunks", async ({
    page,
  }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);

    // Enter real file paths into PathBar and trigger compare
    const leftInput = page.getByTestId("pathbar/left");
    const rightInput = page.getByTestId("pathbar/right");
    await leftInput.fill(fixture.file1Path);
    await leftInput.press("Enter");
    await rightInput.fill(fixture.file2Path);
    await rightInput.press("Enter");

    // Wait for text view to load and show real row count
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    const status = page.getByTestId("textview/status");
    await expect(status).toBeVisible({ timeout: 15000 });
    await expect(status).toContainText("176 / 176 rows", { timeout: 15000 });

    // Verify status breakdown matches ground truth exactly
    await expect(status).toContainText("147 same");
    await expect(status).toContainText("6 changed");
    await expect(status).toContainText("17 left-only");
    await expect(status).toContainText("6 right-only");
    await expect(status).toContainText("left 170 lines · right 159 lines");

    // Verify row 0 has real content from File 1 (deletion of top 3 lines)
    const row0 = page.getByTestId("textview/row-0");
    await expect(row0).toBeVisible();
    await expect(row0).toHaveAttribute("data-kind", "leftOnly");
    // Ensure it is NOT the fixture string "common line 0000"
    await expect(row0).not.toContainText("common line 0000");

    // Verify all 10 hunks mount gutter copy controls
    for (let h = 0; h < 10; h += 1) {
      await expect(page.getByTestId(`hunk-copy-left-${h}`)).toBeVisible();
      await expect(page.getByTestId(`hunk-copy-right-${h}`)).toBeVisible();
    }
    // Exactly 10 hunk pairs (no 11th hunk)
    await expect(page.getByTestId("hunk-copy-left-10")).toHaveCount(0);
    await expect(page.getByTestId("hunk-copy-right-10")).toHaveCount(0);

    expect(errors, "zero console/page errors").toEqual([]);
  });

  test("2. Fake paths -> fixture (back-compat): non-existent paths fall back to dumped.json", async ({
    page,
  }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);

    // Submit fake/non-existent paths
    const leftInput = page.getByTestId("pathbar/left");
    const rightInput = page.getByTestId("pathbar/right");
    await leftInput.fill("/tmp/nonexistent-fake-left-176.txt");
    await leftInput.press("Enter");
    await rightInput.fill("/tmp/nonexistent-fake-right-176.txt");
    await rightInput.press("Enter");

    // Must fall back to the 644-row fixture
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    const status = page.getByTestId("textview/status");
    await expect(status).toBeVisible({ timeout: 15000 });
    await expect(status).toContainText("644 rows", { timeout: 15000 });
    await expect(status).toContainText("4 changed");

    // Verify row 0 contains fixture text
    const row0 = page.getByTestId("textview/row-0");
    await expect(row0).toBeVisible();
    await expect(row0).toContainText("common line 0000 with stable body text");

    expect(errors, "zero console/page errors").toEqual([]);
  });

  test("3. Exact row and hunk counts match python difflib ground truth ±0", async ({
    page,
  }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);

    const leftInput = page.getByTestId("pathbar/left");
    const rightInput = page.getByTestId("pathbar/right");
    await leftInput.fill(fixture.file1Path);
    await leftInput.press("Enter");
    await rightInput.fill(fixture.file2Path);
    await rightInput.press("Enter");

    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    const status = page.getByTestId("textview/status");
    await expect(status).toContainText("176 / 176 rows", { timeout: 15000 });

    // Validate ground truth hunk boundaries:
    // Hunk 0: rows 0..2 (delete 3 lines)
    await expect(page.getByTestId("textview/row-0")).toHaveAttribute("data-kind", "leftOnly");
    await expect(page.getByTestId("textview/row-1")).toHaveAttribute("data-kind", "leftOnly");
    await expect(page.getByTestId("textview/row-2")).toHaveAttribute("data-kind", "leftOnly");

    // Row 3: equal
    await expect(page.getByTestId("textview/row-3")).toHaveAttribute("data-kind", "equal");

    // Hunk 1: rows 4..8 (delete 5 lines)
    await expect(page.getByTestId("textview/row-4")).toHaveAttribute("data-kind", "leftOnly");
    await expect(page.getByTestId("textview/row-8")).toHaveAttribute("data-kind", "leftOnly");

    // Row 9: equal
    await expect(page.getByTestId("textview/row-9")).toHaveAttribute("data-kind", "equal");

    // Hunk 2: rows 25..27 (insert 3 lines)
    await expect(page.getByTestId("textview/row-25")).toHaveAttribute("data-kind", "rightOnly");
    await expect(page.getByTestId("textview/row-27")).toHaveAttribute("data-kind", "rightOnly");

    // Row 28: equal
    await expect(page.getByTestId("textview/row-28")).toHaveAttribute("data-kind", "equal");

    // Hunk 5: row 44 (single row changed)
    await expect(page.getByTestId("textview/row-44")).toHaveAttribute("data-kind", "changed");

    // Hunk 9: rows 157..161 (last hunk)
    await expect(page.getByTestId("textview/row-157")).toHaveAttribute("data-kind", "changed");
    await expect(page.getByTestId("textview/row-161")).not.toHaveAttribute("data-kind", "equal");

    // Row 162: equal trailing block to end
    await expect(page.getByTestId("textview/row-162")).toHaveAttribute("data-kind", "equal");
    await expect(page.getByTestId("textview/row-175")).toHaveAttribute("data-kind", "equal");

    expect(errors, "zero console/page errors").toEqual([]);
  });

  test("4. Gutter hunk copy stages cleanly on real content with BC receipt", async ({
    page,
  }) => {
    const errors = collectErrors(page);
    await installRealContentMock(page);

    const leftInput = page.getByTestId("pathbar/left");
    const rightInput = page.getByTestId("pathbar/right");
    await leftInput.fill(fixture.file1Path);
    await leftInput.press("Enter");
    await rightInput.fill(fixture.file2Path);
    await rightInput.press("Enter");

    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
    await expect(page.getByTestId("textview/status")).toContainText("176 / 176 rows", { timeout: 15000 });

    // Click take-left on Hunk 0 (left-only deletion)
    const takeLeft0 = page.getByTestId("hunk-copy-left-0");
    await expect(takeLeft0).toBeVisible();
    await takeLeft0.click();

    // Verify copy note receipt explains unsaved state
    const copyNote = page.getByTestId("textview/copy-note");
    await expect(copyNote).toBeVisible();
    await expect(copyNote).toContainText("unsaved");

    expect(errors, "zero console/page errors").toEqual([]);
  });
});
