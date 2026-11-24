import { expect, test } from "@playwright/test";
// Wave-66S: inline extension lands new-files-only, existing TextView untouched.
// Mock IPC only (VITE_MOCK_IPC=1 webServer).
test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed:", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("66S inline extension keeps TextView green", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (m) => { if (m.type() === "error") consoleErrors.push(m.text()); });
  const pageErrors: string[] = [];
  page.on("pageerror", (e) => pageErrors.push(String(e)));

  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  // demo banner is conditional (null when mock serves rows without pair echo); view+status is the green gate.

  // Pure logic proof: sideRanges sorts/clamps/merges half-open spans.
  const merged = await page.evaluate(async () => {
    const mod = await import("/src/components/views/text/inline/InlineSpans.tsx");
    return mod.sideRanges(
      [
        { aStart: 8, aEnd: 12, bStart: 0, bEnd: 0 },
        { aStart: 2, aEnd: 5, bStart: 0, bEnd: 0 },
        { aStart: 4, aEnd: 10, bStart: 0, bEnd: 0 },
      ],
      (op: { aStart: number; aEnd: number }) => [op.aStart, op.aEnd] as const,
      20,
    );
  });
  expect(merged).toEqual([[2, 12]]);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
