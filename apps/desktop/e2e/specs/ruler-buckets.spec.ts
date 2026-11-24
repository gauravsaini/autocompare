/**
 * Wave-15 ruler-buckets proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * The wave-13 ruler rendered one marker div per diff row; at 100k+ rows
 * that is 100k+ DOM nodes. Wave-15 caps rendered markers at
 * `MAX_RULER_BUCKETS` (400): row indices are partitioned into contiguous
 * buckets, each bucket keeping `data-testid="ruler/row-<firstIndex>"`
 * and the worst kind in its range (`chg` > `del`/`ins` > `same`). The
 * click path is untouched (`fractionToRowIndex`), so click accuracy is
 * preserved; `role="slider"`, keyboard, `onJump` and the active-index
 * contract are unchanged.
 *
 * This spec drives the REAL `App` workspace (auto-booted demo text
 * session over the mock `textRows` fixture: 644 rows, i.e. past the 400
 * cap) and proves on the live DOM:
 *  - marker count is <= 400 AND > 1 (compression actually engaged);
 *  - more than one kind is rendered and every marker keeps the
 *    `ruler/row-<index>` testid shape with a numeric first index;
 *  - click-jump near the ruler bottom scrolls the virtualized pane
 *    (`textview/virtual` `scrollTop` increases);
 *  - keyboard (`End` / `Home` on the focused `slider`) still jumps;
 *  - light + dark PNGs land in `/tmp/ui-15/` (never in-repo);
 *  - zero console errors / pageerrors.
 *
 * The mock IPC fixture tops out at 644 rows, so the >= 2000-row case is
 * proved against the REAL shipped bucket helper (`compressRowsToBuckets`
 * imported from the component module): a synthetic 5000-row diff
 * compresses to <= 400 contiguous buckets covering every row, with
 * worst-kind propagation intact. Skip policy matches the other specs
 * (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";
import {
  compressRowsToBuckets,
  fractionToRowIndex,
  MAX_RULER_BUCKETS,
  type DiffRow,
} from "../../src/components/chrome/OverviewRuler";

const EVID_DIR = "/tmp/ui-15";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function scrollTopOf(page: Page, testid: string): Promise<number> {
  return page.evaluate((id) => {
    const el = document.querySelector(`[data-testid="${id}"]`);
    return el instanceof HTMLElement ? el.scrollTop : -1;
  }, testid);
}

test("ruler buckets: capped markers, kinds, click-jump scrolls, keyboard moves, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Real App demo text session (mock text_compare over the fixture).
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await expect(page.getByTestId("view-text")).toBeVisible();

  const ruler = page.getByTestId("ruler");
  await expect(ruler).toBeVisible();
  await expect(ruler).toHaveAttribute("role", "slider");

  // Load the full diff so the pane virtualizes (> 500 rows) and the
  // ruler owns the whole 644-row map.
  for (let i = 0; i < 10; i += 1) {
    if ((await page.getByTestId("textview/virtual").count()) > 0) break;
    const more = page.getByRole("button", { name: /Load more/ });
    if ((await more.count()) === 0) break;
    await more.first().click();
  }
  await expect(page.getByTestId("textview/virtual")).toBeVisible();

  // Compression engaged: 644 rows collapse under the 400 cap but stay plural.
  const markers = page.locator('[data-testid^="ruler/row-"]');
  const markerCount = await markers.count();
  expect(markerCount, "bucket markers compress 644 rows below the cap").toBeLessThanOrEqual(
    MAX_RULER_BUCKETS,
  );
  expect(markerCount, "bucket markers stay plural on a 644-row diff").toBeGreaterThan(1);
  expect(MAX_RULER_BUCKETS, "cap is exactly 400").toBe(400);

  // Every marker keeps the ruler/row-<firstIndex> shape with a numeric index.
  const testids = await markers.evaluateAll((els) =>
    els.map((el) => el.getAttribute("data-testid") ?? ""),
  );
  expect(testids.length).toBe(markerCount);
  for (const id of testids) {
    expect(id, `marker testid shape (${id})`).toMatch(/^ruler\/row-\d+$/);
  }
  const firstIndices = testids.map((id) => Number(id.slice("ruler/row-".length)));
  expect(
    [...firstIndices].sort((a, b) => a - b),
    "bucket first-indices are unique and ordered",
  ).toEqual(firstIndices);

  // Kinds survive compression: background plus at least one hunk kind.
  const kinds = await markers.evaluateAll((els) =>
    [...new Set(els.map((el) => el.getAttribute("data-kind") ?? ""))].sort(),
  );
  expect(kinds, "bucket kinds include the same background").toContain("same");
  expect(kinds.length, `more than one kind renders (got: ${kinds.join(",")})`).toBeGreaterThan(1);
  for (const kind of kinds) {
    expect(["same", "del", "ins", "chg"], `known kind (${kind})`).toContain(kind);
  }

  // Click-jump near the ruler bottom scrolls the virtualized pane.
  const box = await ruler.boundingBox();
  expect(box, "ruler has a clickable box").not.toBeNull();
  const beforeClick = await scrollTopOf(page, "textview/virtual");
  expect(beforeClick, "virtual scroller exists").toBeGreaterThanOrEqual(0);
  await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height * 0.9);
  await expect
    .poll(async () => scrollTopOf(page, "textview/virtual"), { timeout: 5000 })
    .toBeGreaterThan(beforeClick);

  // Keyboard still jumps: End lands in the last bucket, Home returns to top.
  // Note: App.tsx's scroll tracker derives the active row from
  // floor(scrollTop/span*total), so subpixel clamping can report 642
  // instead of 643 at max scroll; both are inside the final bucket
  // (starts 642, rows 643-644), so assert last-bucket membership.
  await ruler.focus();
  await page.keyboard.press("End");
  await expect
    .poll(async () => scrollTopOf(page, "textview/virtual"), { timeout: 5000 })
    .toBeGreaterThan(beforeClick);
  await expect
    .poll(
      async () => Number(await ruler.getAttribute("aria-valuenow")),
      { timeout: 5000 },
    )
    .toBeGreaterThanOrEqual(642);
  await page.keyboard.press("Home");
  await expect
    .poll(async () => scrollTopOf(page, "textview/virtual"), { timeout: 5000 })
    .toBe(0);
  await expect(ruler).toHaveAttribute("aria-valuenow", "0");
  // Active highlight follows the jump target bucket.
  await expect(page.locator('[data-testid^="ruler/row-"][data-active="true"]')).toHaveCount(1);

  // Pure-logic proof on a >= 2000-row diff (mock tops out at 644 rows):
  // the shipped helper compresses a synthetic 5000-row diff.
  const big: DiffRow[] = Array.from({ length: 5000 }, (_, i) => ({
    kind: (i === 2500 ? "chg" : i % 97 === 0 ? "del" : i % 53 === 0 ? "ins" : "same") as DiffRow["kind"],
  }));
  const buckets = compressRowsToBuckets(big);
  expect(buckets.length, "5000 rows compress under the cap").toBeLessThanOrEqual(400);
  expect(buckets.length, "5000 rows stay plural").toBeGreaterThan(1);
  expect(buckets[0]?.start, "coverage starts at row 0").toBe(0);
  expect(buckets[buckets.length - 1]?.end, "coverage ends at the last row").toBe(4999);
  for (let i = 1; i < buckets.length; i += 1) {
    expect(buckets[i]?.start, `bucket ${i} is contiguous`).toBe((buckets[i - 1]?.end ?? -1) + 1);
  }
  const chgBucket = buckets.find((b) => b.start <= 2500 && 2500 <= b.end);
  expect(chgBucket?.kind, "worst kind (chg) propagates to its bucket").toBe("chg");
  // Click path unchanged: fractional mapping still resolves row-accurately.
  expect(fractionToRowIndex(0.5, 5000), "fraction 0.5 resolves to row 2500").toBe(2500);
  expect(fractionToRowIndex(0, 5000), "fraction 0 resolves to row 0").toBe(0);

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/ruler-buckets-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/ruler-buckets-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
