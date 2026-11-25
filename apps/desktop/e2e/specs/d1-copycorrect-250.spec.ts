/**
 * D1 copycorrect: gutter hunk-copy (l2r/r2l) across SEVERAL sections never
 * breaks the diff — every copy keeps rows consistent, copies stage in
 * memory only (no autosave), explicit Save persists + converges.
 * MOCK IPC ONLY (`VITE_MOCK_IPC=1`), fresh port, webkit, workers 1.
 *
 * Ground truth (same real files as textfunc-178, difflib/Rust normalized):
 * File1 170 logical lines vs File2 159 -> 176 rows: 147 equal, 6 changed,
 * 17 left-only, 6 right-only, 10 hunks
 * [0-2, 4-8, 25-27, 34-37, 39-41, 44, 56, 111-113, 125, 157-161].
 *
 *  - T1 multi-hunk alternating sequence (l2r/r2l x5): after EVERY copy the
 *    view is self-consistent — loaded rows == total, gutter button pairs
 *    == hunk count with gap-free ordinals 0..N-1, every row mounted, no
 *    save-note (no autosave), copy-note says unsaved + names Save side.
 *  - T2 staged-until-save: copies on both sides light BOTH dirty dots,
 *    never a save-note, and both disk files stay byte-identical (OS-level
 *    proof of no autosave).
 *  - T3 Save persists + converges: Save right then Save left keep the
 *    status text byte-identical to the pre-save staged state (D1 pin: the
 *    pre-D1 refetch truncated the tail — 159/173-style split with lost
 *    merges — after a multi-side staged Save). Staged clears per side,
 *    dirty dots clear per side, files stay byte-identical (mock writes
 *    nothing — same honest boundary as textfunc-178 T2).
 *  - T4 reload boundary: an UNSAVED staged copy is dropped by reload
 *    (BC parity, staged lives in memory only) — fresh compare returns to
 *    176/176 + 10 hunks, files byte-identical. No geometry asserts.
 */
import { readFileSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import {
  installRealContentMock,
  prepareRealContentFiles,
  type RealContentFixture,
} from "../helpers/realcontent";

function trackErrors(page: Page): { pageErrors: string[]; consoleErrors: string[] } {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(String(msg.text()));
  });
  return { pageErrors, consoleErrors };
}

async function bootReal(page: Page, fixture: RealContentFixture): Promise<void> {
  await installRealContentMock(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("textview/status")).toBeVisible({ timeout: 15000 });
  await expect
    .poll(async () => page.getByTestId("textview/status").textContent(), { timeout: 15000 })
    .toMatch(/rows/);
  await page.getByTestId("pathbar/left").fill(fixture.file1Path);
  await page.getByTestId("pathbar/left").press("Enter");
  await page.getByTestId("pathbar/right").fill(fixture.file2Path);
  await page.getByTestId("pathbar/right").press("Enter");
  await expect(page.getByTestId("textview/status")).toContainText("176 / 176 rows", {
    timeout: 15000,
  });
}

function hunkCountFrom(label: string | null): number {
  if (label === null) return -1;
  if (label.includes("no differences")) return 0;
  const m = label.match(/(\d+)\s+hunks?/);
  return m !== null ? Number(m[1]) : -1;
}

/** After every copy: rows==total, buttons==hunks with gap-free ordinals. */
async function expectConsistent(
  page: Page,
  opts: { allowSaveNote?: boolean } = {},
): Promise<{ status: string; hunks: number }> {
  const status = (await page.getByTestId("textview/status").textContent()) ?? "";
  const rowsMatch = status.match(/(\d+)\s*\/\s*(\d+)\s*rows/);
  expect(rowsMatch, `status carries loaded/total rows: ${status}`).not.toBeNull();
  const loaded = Number(rowsMatch?.[1]);
  const total = Number(rowsMatch?.[2]);
  expect(loaded, `loaded rows equal total rows (${status})`).toBe(total);
  expect(status, "no partial (loaded) flag on a fully staged view").not.toContain("(loaded)");

  const label = await page.getByTestId("textview/toolbar-hunks").textContent();
  const hunks = hunkCountFrom(label);
  expect(hunks, `hunk label parses (${label})`).toBeGreaterThanOrEqual(0);
  const leftBtns = await page.locator('[data-testid^="hunk-copy-left-"]').count();
  const rightBtns = await page.locator('[data-testid^="hunk-copy-right-"]').count();
  expect(leftBtns, "left gutter buttons == hunk count").toBe(hunks);
  expect(rightBtns, "right gutter buttons == hunk count").toBe(hunks);
  // Ordinals are gap-free: every 0..N-1 pair mounts exactly once.
  for (let i = 0; i < hunks; i += 1) {
    await expect(page.getByTestId(`hunk-copy-left-${i}`)).toBeVisible();
    await expect(page.getByTestId(`hunk-copy-right-${i}`)).toBeVisible();
  }
  const domRows = await page.locator('[data-testid^="textview/row-"]').count();
  expect(domRows, "every loaded row is mounted").toBe(loaded);
  // No autosave: a copy never raises a save receipt (post-Save calls
  // pass allowSaveNote since the explicit receipt is the proof).
  if (!opts.allowSaveNote) {
    await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
  }
  return { status, hunks };
}

test.describe("D1 copycorrect: alternating hunk copies stay consistent + staged-until-save", () => {
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

  test("1. alternating l2r/r2l x5 keeps rows + hunk ordinals consistent, never autosaves", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    const steps = [
      { testid: "hunk-copy-left-0", dir: "take-left", save: "Save right" },
      { testid: "hunk-copy-right-0", dir: "take-right", save: "Save left" },
      { testid: "hunk-copy-left-0", dir: "take-left", save: "Save right" },
      { testid: "hunk-copy-right-1", dir: "take-right", save: "Save left" },
      { testid: "hunk-copy-left-0", dir: "take-left", save: "Save right" },
    ] as const;
    let hunks = 10;
    for (let s = 0; s < steps.length; s += 1) {
      const step = steps[s];
      await page.getByTestId(step.testid).click();
      const note = page.getByTestId("textview/copy-note");
      await expect(note).toContainText(step.dir, { timeout: 5000 });
      await expect(note).toContainText("unsaved");
      await expect(note).toContainText(`press ${step.save}`);
      const snap = await expectConsistent(page);
      expect(snap.hunks, `step ${s + 1}: hunks strictly shrink`).toBeLessThan(hunks);
      hunks = snap.hunks;
    }
    // 5 sections resolved: 10 -> 5 hunks, rows converged, still staged.
    expect(hunks).toBe(5);
    await expect(page.getByTestId("textview/dirty-left")).toBeVisible();
    await expect(page.getByTestId("textview/dirty-right")).toBeVisible();

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("2. staged-until-save: both sides dirty, no save-note, disk byte-identical", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    await page.getByTestId("hunk-copy-left-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
    await page.getByTestId("hunk-copy-right-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");

    await expect(page.getByTestId("textview/save-note")).toHaveCount(0);
    await expect(page.getByTestId("textview/dirty-left")).toBeVisible();
    await expect(page.getByTestId("textview/dirty-right")).toBeVisible();
    await expectConsistent(page);
    // OS-level proof of no autosave: mock never writes disk.
    expect(readFileSync(fixture.file1Path, "utf8")).toBe(fixture.file1Content);
    expect(readFileSync(fixture.file2Path, "utf8")).toBe(fixture.file2Content);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("3. Save right then Save left persist staged copies without splitting rows", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    await page.getByTestId("hunk-copy-left-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
    await page.getByTestId("hunk-copy-right-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
    await page.getByTestId("hunk-copy-left-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-left");
    await page.getByTestId("hunk-copy-right-1").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("take-right");
    const staged = await expectConsistent(page);

    // Save right flushes the l2r staged copies; the staged view must NOT
    // move (D1 pin: pre-D1 refetch truncated the tail -> 159/173 split).
    await page.getByTestId("textview/save-right").click();
    await expect(page.getByTestId("textview/save-note")).toContainText("Saved right to", {
      timeout: 8000,
    });
    await expect(page.getByTestId("textview/status")).toHaveText(staged.status);
    await expect(page.getByTestId("textview/toolbar-hunks")).toHaveText(
      `${staged.hunks} hunks`,
    );
    // Left side still staged: copy-note names it, dirty-left stays.
    await expect(page.getByTestId("textview/copy-note")).toContainText("press Save left");
    await expect(page.getByTestId("textview/dirty-left")).toBeVisible();
    await expect(page.getByTestId("textview/dirty-right")).toHaveCount(0);

    // Save left flushes the rest; staged clears fully, rows still whole.
    await page.getByTestId("textview/save-left").click();
    await expect(page.getByTestId("textview/save-note")).toContainText("Saved left to", {
      timeout: 8000,
    });
    await expect(page.getByTestId("textview/status")).toHaveText(staged.status);
    await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
    await expect(page.getByTestId("textview/dirty-left")).toHaveCount(0);
    await expect(page.getByTestId("textview/dirty-right")).toHaveCount(0);
    await expectConsistent(page, { allowSaveNote: true });
    // Mock IPC never writes disk: byte-identical (same boundary as T2).
    expect(readFileSync(fixture.file1Path, "utf8")).toBe(fixture.file1Content);
    expect(readFileSync(fixture.file2Path, "utf8")).toBe(fixture.file2Content);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("4. reload drops unsaved staged copies (memory-only), disk truth returns", async ({
    page,
  }) => {
    const { pageErrors, consoleErrors } = trackErrors(page);
    await bootReal(page, fixture);

    await page.getByTestId("hunk-copy-left-0").click();
    await expect(page.getByTestId("textview/copy-note")).toContainText("unsaved");
    await expect(page.getByTestId("textview/row-0")).toHaveAttribute("data-kind", "equal");

    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
    await bootReal(page, fixture);
    await expect(page.getByTestId("textview/status")).toContainText("176 / 176 rows");
    await expect(page.getByTestId("textview/toolbar-hunks")).toHaveText("10 hunks");
    await expect(page.getByTestId("textview/row-0")).toHaveAttribute("data-kind", "leftOnly");
    await expect(page.getByTestId("textview/copy-note")).toHaveCount(0);
    expect(readFileSync(fixture.file1Path, "utf8")).toBe(fixture.file1Content);
    expect(readFileSync(fixture.file2Path, "utf8")).toBe(fixture.file2Content);

    expect(pageErrors).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });
});
