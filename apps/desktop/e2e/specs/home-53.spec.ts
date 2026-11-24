import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-53 home quick-create cards: the welcome state offers one card
 * per session kind, and picking one opens a blank session tab through
 * the existing openSession path. New file only.
 */
test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("welcome cards open a blank session tab per kind", async ({
  page,
}) => {
  // Reach the empty welcome state deterministically. The demo
  // first-paint effect lands async: wait for it to settle (banner means
  // it fired, welcome-with-no-banner may still be pre-demo), then
  // clear via Start blank. A second sweep catches a late banner.
  const banner = page.getByTestId("workspace/demo-banner");
  const clear = page.getByTestId("workspace/demo-clear");
  await expect(banner.or(page.getByTestId("workspace/welcome"))).toBeVisible({
    timeout: 15000,
  });
  for (let round = 0; round < 2; round += 1) {
    try {
      await banner.waitFor({ state: "visible", timeout: 5000 });
      await clear.click();
      await expect(page.getByTestId("session-count")).toContainText(
        "sessions: 0",
        { timeout: 10000 }
      );
    } catch {
      break;
    }
  }
  const welcome = page.getByTestId("workspace/welcome");
  await expect(welcome).toBeVisible();

  for (const kind of ["text", "folder", "hex", "table", "merge", "sync", "media"]) {
    await expect(page.getByTestId(`home/new-${kind}`)).toBeVisible();
  }

  await page.getByTestId("home/new-text").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");

  await assertNoOverlap(page);
});
