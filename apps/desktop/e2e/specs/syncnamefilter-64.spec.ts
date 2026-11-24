import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-64 sync step filter: substring match over planned rel_paths,
 * honest list-only note (apply still runs the full plan), clear/Escape
 * restores, misses render zero rows honestly, Cmd+F focuses the box.
 * Purely presentational, no new IPC.
 * New file only, no existing spec touched.
 */
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

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("step filter narrows list only, misses honestly, focuses via cmd+f", async ({ page }) => {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("sync");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-sync")).toBeVisible();

  const rows = page.locator('[data-testid="syncview/row"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  const before = await rows.count();

  const paths = await page.$$eval('[data-testid="syncview/row"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const base = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  const query = base.length >= 3 ? base : first;

  const filter = page.getByTestId("syncview/name-filter");
  await expect(filter).toBeVisible();
  await filter.fill(query);
  const note = page.getByTestId("syncview/name-note");
  await expect(note).toBeVisible();
  await expect(note).toContainText(query);
  await expect(note).toContainText("apply runs the full plan");
  const narrowed = await rows.count();
  expect(narrowed).toBeLessThanOrEqual(before);
  expect(narrowed).toBeGreaterThanOrEqual(1);

  await filter.fill("zzz-no-such-step-xyz");
  await expect(page.locator('[data-testid="syncview/row"]')).toHaveCount(0);
  await expect(note).toContainText("0 of");

  await filter.fill("");
  await expect(note).toBeHidden();
  await expect(page.locator('[data-testid="syncview/row"]')).toHaveCount(before);

  await filter.fill(query);
  await expect(note).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(note).toBeHidden();

  await page.keyboard.press("Meta+f");
  try {
    await expect.poll(() => focusedTestId(page), { timeout: 2000 }).toBe("syncview/name-filter");
  } catch {
    await page.keyboard.press("Meta+f");
    await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("syncview/name-filter");
  }

  await assertNoOverlap(page, 'main, [data-testid^="view-"], [data-testid^="syncview"], [data-testid^="mediaview"]');
});
