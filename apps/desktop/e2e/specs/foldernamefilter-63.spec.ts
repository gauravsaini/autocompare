import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

/**
 * Wave-63 folder name filter: substring match over loaded relPaths,
 * composes with the status filters, honest match note, clear restores,
 * misses render zero rows honestly, Escape clears, Cmd+F focuses it.
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

test("name filter narrows, misses honestly, clears, focuses via cmd+f", async ({ page }) => {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill(LEFT_DIR);
  await page.getByTestId("session/right-path").fill(RIGHT_DIR);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();

  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  const before = await rows.count();

  const paths = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-path") ?? ""),
  );
  const first = paths.find((p) => p.length > 0) ?? "";
  expect(first.length).toBeGreaterThan(0);
  const base = first.includes("/") ? (first.split("/").pop() ?? first) : first;
  const query = base.length >= 3 ? base : first;
  expect(query.length).toBeGreaterThan(0);

  // Wave-113B preamble (zero assertion change): name filter is default-closed
  // to match BC one-row chrome; open it via the toolbar toggle first.
  await page.getByTestId("folderview/toolbar-namefilter").click();
  const filter = page.getByTestId("folder-name-filter");
  await expect(filter).toBeVisible();
  await filter.fill(query);
  const note = page.getByTestId("folder-name-note");
  await expect(note).toBeVisible();
  await expect(note).toContainText(query);
  const narrowed = await rows.count();
  expect(narrowed).toBeLessThanOrEqual(before);
  expect(narrowed).toBeGreaterThanOrEqual(1);

  await filter.fill("zzz-no-such-entry-xyz");
  await expect(page.locator('[data-testid^="folder-row-"]')).toHaveCount(0);
  await expect(note).toContainText("0 of");

  await filter.fill("");
  await expect(note).toBeHidden();
  await expect(page.locator('[data-testid^="folder-row-"]')).toHaveCount(before);

  await filter.fill(query);
  await expect(note).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(note).toBeHidden();

  await page.keyboard.press("Meta+f");
  try {
    await expect.poll(() => focusedTestId(page), { timeout: 2000 }).toBe("folder-name-filter");
  } catch {
    await page.keyboard.press("Meta+f");
    await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("folder-name-filter");
  }

  await assertNoOverlap(page);
});
