/**
 * Wave-86 table file-meta line proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> TableView) over the mock table engine (`dumped.json`
 * served with real paging semantics; header `id | name | price`, 550
 * rows, 100 loaded) and proves the Beyond Compare style per-side file
 * meta line (testids `tableview/meta`, `tableview/meta-left`,
 * `tableview/meta-right`, rendered inline in the existing counts line
 * so no new stacked chrome row is added):
 *  - the mock omits `leftMeta`/`rightMeta` (no file stats in mock), so
 *    NOTHING extra renders: `tableview/meta` count is 0, the existing
 *    summary (`table-cols-count`) still shows `100 / 550 rows`, zero
 *    console errors / page errors (no crash, no fabricated values, no
 *    placeholder text);
 *  - the pure exported `formatTableMeta` formatter is proven in-page
 *    (null / undefined / invalid meta -> null = hide; full meta ->
 *    `4.1 KB, 2026-11-20` style size + ISO date; `mtimeUnix: null` ->
 *    size alone);
 *  - chrome budget: with both panels closed the first grid row still
 *    sits within 140px of the table section top (the hidden line adds
 *    no stack);
 *  - light + dark PNGs land in `/tmp/ui-86/` (never in-repo).
 *
 * Wave-85 follow-up: the backend starts echoing `leftMeta`/`rightMeta`
 * on the `table_compare` summary, which activates the render path
 * (this spec then upgrades to pin the visible line). Until then this
 * spec pins the honest fallback green. Skip policy matches the other
 * specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-86";

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

async function createTableSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave86-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave86-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

test("table meta: honest-hidden without backend meta, formatter covers render path", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createTableSession(page);
  await expect(page.getByTestId("table-cols-count")).toContainText("100 / 550 rows");

  // 1. Honest fallback: the mock carries no file meta, so nothing extra renders.
  await expect(page.getByTestId("tableview/meta")).toHaveCount(0);
  await expect(page.getByTestId("tableview/meta-left")).toHaveCount(0);
  await expect(page.getByTestId("tableview/meta-right")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // 2. Pure formatter proof (the render path wave-85 activates).
  const formatted = await page.evaluate(async () => {
    const mod = await import("/src/components/views/TableView.tsx");
    const fmt = mod.formatTableMeta as (
      meta: { bytes?: number; mtimeUnix?: number | null } | null | undefined,
    ) => string | null;
    return {
      nullMeta: fmt(null),
      undefinedMeta: fmt(undefined),
      emptyMeta: fmt({ mtimeUnix: null } as never),
      full: fmt({ bytes: 4200, mtimeUnix: 1790000000 }),
      bytesOnly: fmt({ bytes: 512, mtimeUnix: null }),
      exactBytes: fmt({ bytes: 999, mtimeUnix: null }),
      megabytes: fmt({ bytes: 5 * 1024 * 1024, mtimeUnix: null }),
      invalidBytes: fmt({ bytes: -5, mtimeUnix: 1790000000 }),
      nanBytes: fmt({ bytes: Number.NaN, mtimeUnix: 1790000000 }),
      badTime: fmt({ bytes: 4200, mtimeUnix: Number.NaN }),
    };
  });
  expect(formatted.nullMeta, "null meta hides").toBeNull();
  expect(formatted.undefinedMeta, "undefined meta hides").toBeNull();
  expect(formatted.emptyMeta, "bytes-less meta hides").toBeNull();
  expect(formatted.invalidBytes, "negative bytes hide").toBeNull();
  expect(formatted.nanBytes, "NaN bytes hide").toBeNull();
  expect(formatted.full, "full meta renders").not.toBeNull();
  expect(formatted.full ?? "", "full meta names size").toContain("4.1 KB");
  const expectedDay = new Date(1790000000 * 1000).toISOString().slice(0, 10);
  expect(formatted.full ?? "", "full meta names ISO date").toContain(expectedDay);
  expect(formatted.full ?? "", "full meta joins size and date").toContain(", ");
  expect(formatted.bytesOnly, "mtime null renders size alone").toBe("512 B");
  expect(formatted.exactBytes, "sub-KB renders plain bytes").toBe("999 B");
  expect(formatted.megabytes, "MB renders one decimal").toBe("5.0 MB");
  expect(formatted.badTime, "NaN mtime falls back to size alone").toBe("4.1 KB");

  // 3. Chrome budget: with both panels closed the hidden line adds no stack.
  // Wave-113A note: find panel is already default-closed (no toggle needed).
  await expect(page.getByTestId("table-find-input")).toBeHidden();
  await expect(page.getByTestId("table-cols-menu")).toHaveCount(0);
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { sectionTop: s.top, firstTop: f.top, delta: f.top - s.top };
  });
  expect(chrome, "section + first row mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `first row within collapsed chrome budget (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/tablemeta-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/tablemeta-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
