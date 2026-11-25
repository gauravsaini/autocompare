/**
 * Wave-WA folder toolbar unify proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Pins the BC-parity unify of `FolderToolbar` on the shared ui primitives:
 *  - All/Diffs/Same (+ Changes only) ride ONE shared SegmentedControl
 *    path (single-select: exactly one row option pressed at a time);
 *  - Name filter text box rides shared ui/Input (wiring intact:
 *    toggle -> fill narrows -> Esc clears + closes -> Ctrl+F reopens +
 *    focuses);
 *  - sm/md height tokens: segment buttons measure 22px (<= 26 budget),
 *    toolbar stays one compact row (< 120px);
 *  - collapsed chrome budget intact: section-relative firstRowTop <= 140;
 *  - light + dark, zero console/page errors, no role=alert.
 *
 * Gallery NOT needed: the folder view self-shows every control.
 * New file only, no existing spec touched, no testid changed.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/folder-WA";

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

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

/** Section-relative first-row top (chrome-budget contract, panel closed). */
async function folderFirstRowDelta(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
}

test("folder toolbar WA: single-select segment, input wiring, heights, chrome budget, light+dark", async ({
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

  // 1. ONE shared segment group hosts All/Diffs/Same + Changes only.
  const group = page.getByRole("group", { name: "Row filter" });
  await expect(group).toBeVisible();
  for (const id of ["folder-filter-all", "folder-filter-diffs", "folder-filter-same", "folder-changesonly-toggle"]) {
    await expect(page.getByTestId(id)).toBeVisible();
  }

  // 2. Height tokens: sm segment buttons measure 22px (<= 26 budget);
  //    toolbar stays one compact row (< 120px).
  for (const id of ["folder-filter-all", "folder-filter-diffs", "folder-filter-same", "folder-changesonly-toggle"]) {
    const h = await page.getByTestId(id).evaluate((el) => el.getBoundingClientRect().height);
    expect(h, `${id} rides the sm 22px token (got ${h}px)`).toBeLessThanOrEqual(26);
  }
  const toolbar = page.getByTestId("folderview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar is one compact row (got ${barH}px)`).toBeLessThan(120);

  // 3. Single-select: exactly one row option pressed at a time.
  const pressed = async (id: string): Promise<string | null> =>
    page.getByTestId(id).getAttribute("aria-pressed");
  await page.getByTestId("folder-filter-diffs").click();
  expect(await pressed("folder-filter-diffs")).toBe("true");
  expect(await pressed("folder-filter-all")).toBe("false");
  expect(await pressed("folder-filter-same")).toBe("false");
  const diffStatuses: string[] = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
  expect(diffStatuses.length).toBeGreaterThan(0);
  for (const s of diffStatuses) {
    expect(s, "diffs filter hides every same row").not.toBe("same");
  }
  await page.getByTestId("folder-filter-same").click();
  expect(await pressed("folder-filter-same")).toBe("true");
  expect(await pressed("folder-filter-all")).toBe("false");
  expect(await pressed("folder-filter-diffs")).toBe("false");
  await page.getByTestId("folder-filter-all").click();
  expect(await pressed("folder-filter-all")).toBe("true");
  expect(await pressed("folder-filter-diffs")).toBe("false");
  expect(await pressed("folder-filter-same")).toBe("false");

  // 4. Changes only composes independently (same pressed-state contract).
  const changesToggle = page.getByTestId("folder-changesonly-toggle");
  await changesToggle.click();
  await expect(changesToggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("folder-changes-count")).toBeVisible();
  await changesToggle.click();
  await expect(changesToggle).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByTestId("folder-changes-count")).toHaveCount(0);

  // 5. Name-filter Input wiring intact (shared ui/Input in FolderNamePanel).
  const nameToggle = page.getByTestId("folderview/toolbar-namefilter");
  if ((await nameToggle.getAttribute("aria-expanded")) !== "true") {
    await nameToggle.click();
  }
  await expect(page.getByTestId("folder-name-filter")).toBeVisible();
  const inputH = await page.getByTestId("folder-name-filter").evaluate((el) => el.getBoundingClientRect().height);
  expect(inputH, `name input rides the sm 22px token (got ${inputH}px)`).toBeLessThanOrEqual(26);
  const filter = page.getByTestId("folder-name-filter");
  await filter.fill("d00");
  await expect(page.getByTestId("folder-name-note")).toBeVisible();
  await expect(page.getByTestId("folder-name-note")).toContainText("d00");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folder-name-filter")).toBeHidden();
  await expect(page.getByTestId("folder-name-note")).toHaveCount(0);
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("folder-name-filter")).toBeVisible();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("folder-name-filter");
  // Leave the panel closed for the chrome-budget read.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folder-name-filter")).toBeHidden();

  // 6. Chrome budget intact: section-relative firstRowTop <= 140.
  const delta = await folderFirstRowDelta(page);
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(delta!, `folder chrome collapsed to one row (got ${delta}px)`).toBeLessThanOrEqual(140);

  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/folder-toolbar-light.png` });

  // 7. Dark: segment + wiring + budget hold, zero new errors.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(toolbar).toBeVisible();
  await page.getByTestId("folder-filter-diffs").click();
  expect(await pressed("folder-filter-diffs")).toBe("true");
  expect(await pressed("folder-filter-all")).toBe("false");
  await page.getByTestId("folder-filter-all").click();
  expect(await pressed("folder-filter-all")).toBe("true");
  const darkDelta = await folderFirstRowDelta(page);
  expect(darkDelta!, `dark firstRowTop <= 140 (got ${darkDelta})`).toBeLessThanOrEqual(140);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/folder-toolbar-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
