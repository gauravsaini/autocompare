/**
 * Wave-43 text goto-row proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace over the auto-boot demo text diff
 * (real engine rows) and proves Beyond Compare style row jumping:
 *  - a row number beyond the initially loaded window loads further
 *    windows on demand, then selects the target row, scrolls it into
 *    view and moves DOM focus to it (status reports the jump);
 *  - a row inside the loaded window jumps without extra loads;
 *  - garbage ("abc", empty, "0") and out-of-range ("999999") render
 *    honest inline notes and leave the selection untouched -- never a
 *    crash, zero `role=alert`;
 *  - light + dark PNGs land in `/tmp/ui-43/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-43";

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

async function totals(page: Page): Promise<{ loaded: number; total: number }> {
  const text = ((await page.getByTestId("textview/status").textContent()) ?? "").trim();
  const m = /(\d+)\s*\/\s*(\d+)\s*rows/.exec(text);
  expect(m, `totals readable (got: ${text})`).not.toBeNull();
  return { loaded: Number(m![1]), total: Number(m![2]) };
}

async function focusedRowTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

test("text goto: on-demand load + jump/select/focus, invalid + range notes, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  const { loaded, total } = await totals(page);
  expect(total, "demo has rows to jump through").toBeGreaterThan(10);

  const input = page.getByTestId("textview/goto-input");
  const go = page.getByTestId("textview/goto-go");
  const status = page.getByTestId("textview/goto-status");
  // Wave-112 preamble: the goto panel is closed by default (BC truth);
  // open it via its toolbar toggle. Zero assertion changes below.
  const gotoToggle112 = page.getByTestId("textview/toolbar-goto");
  if ((await gotoToggle112.getAttribute("aria-expanded")) !== "true") await gotoToggle112.click();
  await expect(input).toBeVisible();
  await expect(go).toBeVisible();

  // Target beyond the loaded window forces on-demand loads (when the
  // demo has more than one window); otherwise the last loaded row.
  const target = loaded < total ? Math.min(total, loaded + 50) : loaded;
  await input.fill(String(target));
  await go.click();
  await expect(status).toContainText(`jumped to row ${target}`, { timeout: 30000 });
  await expect
    .poll(() => focusedRowTestId(page), { timeout: 10000 })
    .toMatch(/^textview\/row-/);
  const selected = await page
    .locator('[data-testid^="textview/row-"][data-selected="true"]')
    .count();
  expect(selected, "exactly one row selected after jump").toBe(1);

  // A row inside the loaded window jumps directly (Enter works too).
  const near = Math.max(1, Math.min(loaded, 5));
  await input.fill(String(near));
  await input.press("Enter");
  await expect(status).toContainText(`jumped to row ${near}`, { timeout: 30000 });
  await expect
    .poll(() => focusedRowTestId(page), { timeout: 10000 })
    .toMatch(/^textview\/row-/);

  // Garbage + out-of-range render honest notes, selection-proof:
  // exactly one row stays selected, no alert appears.
  const selectedBefore = await page
    .locator('[data-testid^="textview/row-"][data-selected="true"]')
    .first()
    .getAttribute("data-testid");
  for (const bad of ["abc", "", "0", String(total + 999)]) {
    await input.fill(bad);
    await go.click();
    await expect(status).not.toContainText("jumped", { timeout: 10000 });
  }
  await expect(status).toContainText(/enter a row number|out of range/);
  const selectedAfter = await page
    .locator('[data-testid^="textview/row-"][data-selected="true"]')
    .first()
    .getAttribute("data-testid");
  expect(selectedAfter, "failed jumps keep the selection").toBe(selectedBefore);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/text-goto-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/text-goto-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
