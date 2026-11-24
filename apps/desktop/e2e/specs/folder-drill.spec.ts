/**
 * Wave-46 folder drilldown proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> folder tab)
 * and proves Beyond Compare style drilldown:
 *  - double-clicking a two-sided FILE row opens its pair in a NEW text
 *    session (a second tab appears, the text view shows the joined
 *    `leftDir/relPath` pair, the folder tab is untouched);
 *  - double-clicking a one-sided row (leftOnly/rightOnly, when the
 *    fixture has one) renders an honest inline note
 *    (`folder-open-note`) and opens nothing;
 *  - light + dark PNGs land in `/tmp/ui-46/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/ui-46";

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

test("folder drilldown: dblclick two-sided file opens text tab, one-sided notes honestly, light+dark", async ({
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

  // First page is all one-sided by engine order; page once to reach the
  // first two-sided files (index 57+). Bounded, same as folder-real.
  for (let i = 0; i < 3; i += 1) {
    const two = page.locator(
      '[data-testid^="folder-row-"][data-status="same"], [data-testid^="folder-row-"][data-status="different"]',
    );
    if ((await two.count()) > 0) break;
    const more = page.getByTestId("folderview/load-more");
    if ((await more.count()) === 0) break;
    await more.first().click();
    await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  }
  // Two-sided file row drills into a NEW text session tab (first loaded
  // two-sided file, no paging assumption).
  const fileRow = page
    .locator(
      '[data-testid^="folder-row-"][data-status="same"], [data-testid^="folder-row-"][data-status="different"]',
    )
    .first();
  await expect(fileRow).toHaveCount(1);
  const rel = (await fileRow.getAttribute("data-path")) ?? "";
  expect(rel.length, "two-sided row has a path").toBeGreaterThan(0);
  const tabsBefore = await page.getByRole("tab").count();
  await fileRow.dblclick();
  await expect
    .poll(async () => page.getByRole("tab").count(), { timeout: 10000 })
    .toBeGreaterThan(tabsBefore);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("pathbar/left")).toHaveValue(new RegExp(rel.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "$"));
  await expect(page.getByTestId("pathbar/right")).toHaveValue(new RegExp(rel.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "$"));

  // Back to the folder tab: it is untouched (still lists rows). The
  // workspace may already hold a demo tab, so walk tabs until the folder
  // view reappears instead of assuming index 0.
  {
    const tabs = page.getByRole("tab");
    const n = await tabs.count();
    let found = false;
    for (let i = n - 1; i >= 0; i -= 1) {
      await tabs.nth(i).click();
      try {
        await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 2000 });
        found = true;
        break;
      } catch {
        continue;
      }
    }
    expect(found, "folder tab still present after drilldown").toBe(true);
  }
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // One-sided row (when the fixture has one): honest note, no new tab.
  const oneSided = page.locator(
    '[data-testid^="folder-row-"][data-status="leftOnly"][role="button"], [data-testid^="folder-row-"][data-status="rightOnly"][role="button"]',
  );
  if ((await oneSided.count()) > 0) {
    const tabsMid = await page.getByRole("tab").count();
    await oneSided.first().dblclick();
    await expect(page.getByTestId("folder-open-note")).toContainText(/one-sided row/);
    expect(await page.getByRole("tab").count(), "one-sided opens nothing").toBe(
      tabsMid,
    );
  }

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folder-drill-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/folder-drill-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
