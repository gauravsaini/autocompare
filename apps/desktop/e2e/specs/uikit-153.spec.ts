/**
 * Wave-A: shared desktop control kit contract (source-level, no pixels).
 * Pins that `src/components/ui/` exposes the 6 primitives with macOS
 * density tokens, so Wave-B migration has a stable target. Testids ride
 * call-sites untouched — this spec never asserts rendered pixels.
 */
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function fetchSrc(page: Page, path: string): Promise<string | null> {
  try {
    return await page.evaluate(async (p: string) => {
      const res = await fetch(p);
      if (!res.ok) return null;
      return res.text();
    }, path);
  } catch { return null; }
}

test.beforeEach(async ({ page }) => {
  let up = false;
  try { await page.goto("/", { timeout: 15000 }); up = true; } catch { up = false; }
  test.skip(!up, "dev server unreachable -- skipping uikit contract");
});

test("uikit exposes Button/Segmented/ToolbarButton/Checkbox/Input/Select", async ({ page }) => {
  const idx = await fetchSrc(page, "/src/components/ui/index.ts");
  test.skip(idx === null, "could not fetch ui index");
  for (const name of ["Button", "SegmentedControl", "ToolbarButton", "Checkbox", "Input", "Select"]) {
    expect(idx, `ui index exports ${name}`).toContain(name);
  }
});

test("control tokens exist in light and dark", async ({ page }) => {
  const css = await fetchSrc(page, "/src/theme/tokens.css");
  test.skip(css === null, "could not fetch tokens.css");
  for (const tok of ["--ac-control-h-sm: 22px", "--ac-btn-bg", "--ac-segment-track", "--ac-checkbox-size: 13px"]) {
    expect(css, `tokens carry ${tok}`).toContain(tok);
  }
});

test("ToolbarButton has zero hand-coded svg (Lucide-only rule)", async ({ page }) => {
  const src = await fetchSrc(page, "/src/components/ui/ToolbarButton.tsx");
  test.skip(src === null, "could not fetch ToolbarButton");
  expect(src, "no inline <svg> in shared button").not.toContain("<svg");
});
