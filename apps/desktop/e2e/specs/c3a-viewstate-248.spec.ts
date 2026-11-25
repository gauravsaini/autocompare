/**
 * Wave-C3a view-state persistence (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Har view ke compare options browser reload ke baad survive karte hain
 * (localStorage `ac.viewstate.v1.<view>`, JSON, corrupt = defaults):
 *  - text: ignore-case + encoding + patience toggle set karo -> reload ->
 *    controls abhi bhi set; corrupt storage -> defaults wapas;
 *  - table: ignore-case + delimiter + epsilon set karo -> reload -> naya
 *    table session banao -> controls abhi bhi set; corrupt -> defaults;
 *  - folder: masks apply karo -> reload -> naya folder session banao ->
 *    rules draft me masks abhi bhi; corrupt -> khaali.
 *
 * Sirf control state assert hoti hai (koi geometry assert nahi, koi
 * screenshot nahi). Skip policy baaki specs jaisi (infra fail = skip).
 * Nayi file; koi existing spec nahi chhedi.
 */
import { expect, test, type Page } from "@playwright/test";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function assertClean(consoleErrors: string[], pageErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

async function openTextIgnore(page: Page): Promise<void> {
  const toggle = page.getByTestId("textview/toolbar-ignore");
  if ((await toggle.getAttribute("aria-expanded")) !== "true") await toggle.click();
  await expect(page.getByTestId("textview/ignore-case")).toBeVisible();
}

async function openTextFormat(page: Page): Promise<void> {
  const toggle = page.getByTestId("textview/toolbar-format");
  if ((await toggle.getAttribute("aria-expanded")) !== "true") await toggle.click();
  await expect(page.getByTestId("textview/algorithm-patience")).toBeVisible();
}

async function createTableSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave248-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave248-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

async function createFolderSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill("/tmp/wave17-left-dir");
  await page.getByTestId("session/right-path").fill("/tmp/wave17-right-dir");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible();
}

test("c3a text view-state: options survive reload, corrupt falls back to defaults", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  // Demo text diff auto-boots (mock IPC, real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 1. Options set karo: ignore-case + patience + encoding.
  await openTextIgnore(page);
  const ignoreCase = page.getByTestId("textview/ignore-case");
  await expect(ignoreCase).not.toBeChecked();
  await ignoreCase.check();
  await expect(ignoreCase).toBeChecked();
  await page.keyboard.press("Escape");
  await expect(ignoreCase).toBeHidden();

  await openTextFormat(page);
  const patience = page.getByTestId("textview/algorithm-patience");
  const encoding = page.getByTestId("textview/format-encoding");
  await expect(patience).not.toBeChecked();
  await patience.check();
  await expect(patience).toBeChecked();
  await encoding.selectOption("windows-1252");
  await expect(encoding).toHaveValue("windows-1252");
  await page.keyboard.press("Escape");
  await expect(patience).toBeHidden();

  // 2. Reload -> options abhi bhi set.
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  await openTextIgnore(page);
  await expect(page.getByTestId("textview/ignore-case")).toBeChecked();
  await page.keyboard.press("Escape");

  await openTextFormat(page);
  await expect(page.getByTestId("textview/algorithm-patience")).toBeChecked();
  await expect(page.getByTestId("textview/format-encoding")).toHaveValue("windows-1252");
  await page.keyboard.press("Escape");

  // 3. Corrupt storage -> defaults wapas.
  await page.evaluate(() => {
    window.localStorage.setItem("ac.viewstate.v1.text", "{corrupt!!");
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("view-text")).toBeVisible();

  await openTextIgnore(page);
  await expect(page.getByTestId("textview/ignore-case")).not.toBeChecked();
  await page.keyboard.press("Escape");

  await openTextFormat(page);
  await expect(page.getByTestId("textview/algorithm-patience")).not.toBeChecked();
  await expect(page.getByTestId("textview/format-encoding")).toHaveValue("");
  await page.keyboard.press("Escape");

  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});

test("c3a table view-state: options survive reload, corrupt falls back to defaults", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  await createTableSession(page);
  const view = page.getByTestId("view-table");
  await expect(view).toContainText("100 / 550 rows");

  // 1. Options set karo.
  const ignoreCase = page.getByTestId("tableview/ignore-case");
  const delimiter = page.getByTestId("tableview/delimiter-select");
  const epsilon = page.getByTestId("tableview/numeric-tolerance");
  await ignoreCase.check();
  await expect(ignoreCase).toBeChecked();
  await delimiter.selectOption(";");
  await expect(delimiter).toHaveValue(";");
  await epsilon.fill("0.01");
  await expect(epsilon).toHaveValue("0.01");

  // 2. Reload (sessions in-memory hain, view wapas demo par) -> naya
  // table session banao -> options abhi bhi set.
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await createTableSession(page);
  await expect(view).toContainText("100 / 550 rows");
  await expect(page.getByTestId("tableview/ignore-case")).toBeChecked();
  await expect(page.getByTestId("tableview/delimiter-select")).toHaveValue(";");
  await expect(page.getByTestId("tableview/numeric-tolerance")).toHaveValue("0.01");

  // 3. Corrupt storage -> defaults wapas.
  await page.evaluate(() => {
    window.localStorage.setItem("ac.viewstate.v1.table", "[1,2,3]");
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await createTableSession(page);
  await expect(page.getByTestId("tableview/ignore-case")).not.toBeChecked();
  await expect(page.getByTestId("tableview/delimiter-select")).toHaveValue(",");
  await expect(page.getByTestId("tableview/numeric-tolerance")).toHaveValue("");

  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});

test("c3a folder view-state: masks survive reload, corrupt falls back to defaults", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  await createFolderSession(page);
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);

  // 1. Masks apply karo.
  await page.getByTestId("folderview/toolbar-rules").click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  await page.getByTestId("folderview/mask-input").fill("*.log");
  await page.getByTestId("folderview/mask-apply").click();
  await expect(page.getByTestId("folderview/masks-note")).toContainText("*.log");

  // 2. Reload -> naya folder session banao -> masks abhi bhi set.
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await createFolderSession(page);
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  await page.getByTestId("folderview/toolbar-rules").click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  await expect(page.getByTestId("folderview/mask-input")).toHaveValue("*.log");
  await page.keyboard.press("Escape");

  // 3. Corrupt storage (galat type bhi) -> defaults wapas.
  await page.evaluate(() => {
    window.localStorage.setItem("ac.viewstate.v1.folder", '{"masks": 42}');
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  await createFolderSession(page);
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBeGreaterThan(0);
  await page.getByTestId("folderview/toolbar-rules").click();
  await expect(page.getByTestId("folderview/rules-panel")).toBeVisible();
  await expect(page.getByTestId("folderview/mask-input")).toHaveValue("");

  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});
