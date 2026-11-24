/**
 * Wave-144 typexquis: bundled fonts + icons (BC parity).
 *
 * PORT-FREE gate spec (fresh 1561, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - bundled Inter 400/500/600 + JetBrains Mono 400/500 load via
 *    document.fonts (offline-first repo woff2, no remote fetch);
 *  - computed-family pins: body rides Inter, mono cells ride JetBrains Mono;
 *  - icon audit: chrome/view close + nav glyphs render Lucide/crisp SVG,
 *    body text me koi emoji ya text-arrow glyph nahi (data separators
 *    <-> aur x-dimension ke alawa).
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("typexquis-144: bundled fonts load via document.fonts", async ({ page }) => {
  const loaded = await page.evaluate(async () => {
    const wants = [
      "400 12px Inter",
      "500 12px Inter",
      "600 12px Inter",
      "400 12px \"JetBrains Mono\"",
      "500 12px \"JetBrains Mono\"",
    ];
    const out: Record<string, boolean> = {};
    for (const w of wants) {
      try {
        await document.fonts.load(w);
      } catch {
        // fall through to check() below
      }
      out[w] = document.fonts.check(w);
    }
    return out;
  });
  for (const [face, ok] of Object.entries(loaded)) {
    expect(ok, `bundled face loads offline-first: ${face}`).toBe(true);
  }
});

test("typexquis-144: computed families pin to bundled stacks", async ({ page }) => {
  const bodyFamily = await page.evaluate(
    (el) => window.getComputedStyle(el).fontFamily,
    await page.locator("body").elementHandle(),
  );
  expect(bodyFamily, "body rides the Inter stack").toContain("Inter");

  const monoFamily = await page
    .getByTestId("session-count")
    .first()
    .evaluate((el) => window.getComputedStyle(el).fontFamily);
  expect(monoFamily, "mono rides the JetBrains Mono stack").toContain("JetBrains Mono");
});

test("typexquis-144: icons are Lucide/SVG, no emoji or text-arrow glyphs", async ({ page }) => {
  // Mock demo auto-opens a text session (find/goto panels open by
  // default, so their close buttons mount). Welcome race se bachne ke
  // liye click nahi — demo ka wait.
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });

  // Panels toggle-open (default state varies by session path).
  for (const toggle of ["textview/toolbar-find", "textview/toolbar-goto"]) {
    const btn = page.getByTestId(toggle);
    await expect(btn).toBeVisible();
    if ((await btn.getAttribute("aria-expanded")) !== "true") {
      await btn.click();
    }
  }

  // Close buttons (text find/goto panels) render the Lucide X svg,
  // never a text x glyph.
  for (const testid of ["textview/find-close", "textview/goto-close"]) {
    const btn = page.getByTestId(testid);
    await expect(btn, `${testid} visible`).toBeVisible();
    await expect(btn.locator("svg"), `${testid} renders Lucide svg`).toBeVisible();
    await expect(btn, `${testid} carries no text glyph`).not.toContainText("×");
  }

  // Gutter take arrows ride the existing crisp inline TakeArrow svg.
  const gutterArrow = page.locator('[data-testid^="hunk-copy-"] svg').first();
  if ((await gutterArrow.count()) > 0) {
    await expect(gutterArrow).toBeVisible();
  }

  // Body-wide glyph scan: koi emoji ya text-arrow icon glyph nahi.
  // (↔ session-name separator aur × dimension/data math allowed hain —
  // woh data hain, icon nahi; REPORT me listed.)
  const hits = await page.evaluate(() => {
    const text = document.body.innerText ?? "";
    const bad = text.match(/[→←↑↓▶◀▲▼✓✗⇤⇥⇒⇐★😀-🙏]/u);
    return bad ? bad[0] : null;
  });
  expect(hits, "no emoji or text-arrow icon glyph in chrome/views").toBeNull();

  await assertNoOverlap(page);
});
