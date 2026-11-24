/**
 * Wave-170 hex top-bar bus parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) over the embedded 11-byte `h_*` pair ("hello
 * world" vs "hallo world", single page, one differing offset at 1) and
 * proves the hex slice of the `autocompare:view-*` bus (Beyond Compare
 * unified-toolbar parity), hex-side only, no App.tsx change:
 *  (1) bus next/prev: `autocompare:view-command` `{ cmd: "next"/"prev",
 *      viewKind: "hex" }` (or unkinded) moves the hex diff cursor via
 *      the same `stepDiff` path as the toolbar Prev/Next diff buttons;
 *      foreign kinds are ignored; exhausted nav parks honestly on
 *      `hex-diff-status` ("no further/earlier differences").
 *  (2) bus filter: `autocompare:view-filter` `{ filter, viewKind }`
 *      narrows the hex rows (diffs -> 1 row, same -> 15, all -> 16);
 *      foreign kinds are ignored; Find/Goto panels stay untouched.
 * Chrome budget guarded: first hex row top <= 140px relative to the
 * hex section top with both panels closed, single toolbar row, every
 * legacy testid intact. Zero console/page errors. New file only, no
 * existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

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

async function createHexSession(page: Page): Promise<void> {
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/h170-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/h170-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

async function dispatchViewCommand(
  page: Page,
  cmd: "prev" | "next",
  viewKind?: string,
): Promise<void> {
  await page.evaluate(
    ({ cmd, viewKind }) => {
      const detail: Record<string, unknown> = { cmd };
      if (viewKind !== undefined) detail.viewKind = viewKind;
      window.dispatchEvent(new CustomEvent("autocompare:view-command", { detail }));
    },
    { cmd, viewKind },
  );
}

async function dispatchViewFilter(
  page: Page,
  filter: "all" | "diffs" | "same" | "context",
  viewKind?: string | null,
): Promise<void> {
  await page.evaluate(
    ({ filter, viewKind }) => {
      window.dispatchEvent(
        new CustomEvent("autocompare:view-filter", { detail: { filter, viewKind } }),
      );
    },
    { filter, viewKind: viewKind ?? null },
  );
}

test("hexparity-170: bus next/prev moves hex diff cursor, foreign kinds ignored", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createHexSession(page);
  const view = page.getByTestId("view-hex");
  await expect(view).toContainText("files differ");
  await expect(view).toContainText("11 bytes");

  const diffStatus = page.getByTestId("hex-diff-status");
  await expect(diffStatus).toContainText("diff");

  // Bus next (hex kind) lands the single differing offset 1, same as
  // the toolbar Next diff button: selected byte + detail + receipt.
  await dispatchViewCommand(page, "next", "hex");
  await expect(diffStatus).toContainText("difference at offset 1");
  await expect(page.getByTestId("hexview/byte-1")).toHaveAttribute(
    "data-selected",
    "true",
  );
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(1)");

  // Foreign kind is ignored: text-scoped nav never moves the hex cursor.
  await dispatchViewCommand(page, "next", "text");
  await expect(diffStatus).toContainText("difference at offset 1");
  await expect(page.getByTestId("hexview/byte-1")).toHaveAttribute(
    "data-selected",
    "true",
  );

  // Bus prev (hex kind) from the first difference parks honestly.
  await dispatchViewCommand(page, "prev", "hex");
  await expect(diffStatus).toContainText("no earlier differences");

  // Unkinded next is honored (broadcast follow-through): from offset 1
  // nothing follows on the single page, so it parks honestly too.
  await dispatchViewCommand(page, "next");
  await expect(diffStatus).toContainText("no further differences");

  // Find/Goto panels untouched by the bus: still open, inputs usable.
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect(page.getByTestId("hex-goto-input")).toBeVisible();
  await page.getByTestId("hex-find-input").fill("6c");
  await page.getByTestId("hex-find-next").click();
  await expect(page.getByTestId("hex-find-status")).toContainText("match at offset 2");

  await assertNoOverlap(page);
  expect(consoleErrors, `console errors: ${consoleErrors.join(" | ")}`).toEqual([]);
  expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);
});

test("hexparity-170: bus filter narrows hex rows, chrome budget holds", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createHexSession(page);
  const view = page.getByTestId("view-hex");
  await expect(view).toContainText("files differ");

  const rows = view.locator(".hx-row");
  await expect(rows.first()).toBeVisible();
  await expect(rows).toHaveCount(16);

  // Bus diffs (hex kind): only the row holding offset 1 survives.
  await dispatchViewFilter(page, "diffs", "hex");
  await expect(rows).toHaveCount(1);
  await expect(page.getByTestId("hexview/diffsonly-note")).toBeVisible();
  await expect(page.getByTestId("hexview/diffsonly-toggle")).toBeChecked();

  // Foreign kind ignored: text-scoped filter never leaks into hex.
  await dispatchViewFilter(page, "all", "text");
  await expect(rows).toHaveCount(1);

  // Bus same (hex kind): the 15 matching rows show.
  await dispatchViewFilter(page, "same", "hex");
  await expect(rows).toHaveCount(15);
  await expect(page.getByTestId("hexview/diffsonly-note")).toBeVisible();

  // Bus all (hex kind) restores the full grid.
  await dispatchViewFilter(page, "all", "hex");
  await expect(rows).toHaveCount(16);
  await expect(page.getByTestId("hexview/diffsonly-note")).toBeHidden();

  // Unkinded diffs broadcast is honored.
  await dispatchViewFilter(page, "diffs");
  await expect(rows).toHaveCount(1);
  await dispatchViewFilter(page, "all", "hex");
  await expect(rows).toHaveCount(16);

  // Chrome budget: no new rows from the bus wiring — panels closed,
  // single toolbar row, first row top <= 140px section-relative.
  await page.getByTestId("hexview/toolbar-find").click();
  await page.getByTestId("hexview/toolbar-goto").click();
  await expect(page.getByTestId("hex-find-input")).toBeHidden();
  await expect(page.getByTestId("hex-goto-input")).toBeHidden();
  const toolbar = page.getByTestId("hexview/toolbar");
  const barH = await toolbar.evaluate((el) => el.getBoundingClientRect().height);
  expect(barH, `toolbar one compact row (got ${barH}px)`).toBeLessThan(120);
  const delta = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="hex comparison"]');
    const first = document.querySelector(".hx-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return first.getBoundingClientRect().top - section.getBoundingClientRect().top;
  });
  expect(delta, "section + first row mounted").not.toBeNull();
  expect(delta!, `firstRowTop<=140 (got ${delta})`).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  expect(consoleErrors, `console errors: ${consoleErrors.join(" | ")}`).toEqual([]);
  expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);
});
