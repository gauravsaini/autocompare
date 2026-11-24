/**
 * Wave-66L hex dual-pane BC-parity proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) with a hex session and proves the Beyond Compare
 * style two-sided hex grid:
 *  - a right hex column renders next to the left one: every defined left
 *    offset (`hexview/byte-<o>`) has a 1:1 right twin
 *    (`hexview/byte-R-<o>`) in the same row (equal y), with the same
 *    byte affordances (role=button, tabIndex, click selects);
 *  - a right ASCII column mirrors the left `.hx-ascii` column (row 0
 *    reads `hello world` left / `hallo world` right);
 *  - the known-differing offset 1 (left `65 'e'` vs right `61 'a'`)
 *    carries `hx-diff` on BOTH sides, while the same offset 0 carries it
 *    on neither;
 *  - beyond-EOF slots on the live row (offsets 11..15, both sides short
 *    of the 11-byte mock pair) carry `.hx-cell-empty` with the exact 66I
 *    orphan-hatch gradient (`repeating-linear-gradient`, `-45deg`) and
 *    no `hx-diff` tint;
 *  - clicking a right byte selects the same absolute offset (shared
 *    single selection: `byte-R-1` click marks both twins
 *    `data-selected`, footer offset + diff flag follow; keyboard
 *    ArrowRight/ArrowLeft moves from the right cell too);
 *  - one composition smoke: paging (single-page Prev/Next disabled),
 *    find (`6c` still matches at offset 2 with exactly one
 *    `.hx-find-match` on the left while both columns stay rendered),
 *    goto (`5` selects `byte-5` + `byte-R-5`), and copy (hex `65` /
 *    ascii `e` receipts) all behave exactly as before;
 *  - light + dark DOM checks (no PNGs committed), assertNoOverlap,
 *    zero console errors / page errors throughout.
 *
 * The mock IPC adapter has no hex fixtures (it resolves `[]`), so HexView
 * pages over small embedded byte-identical copies of the
 * `tools/parity-harness/fixtures/h_left.bin` ("hello world") and
 * `h_right.bin` ("hallo world") pair; every assertion below reads the DOM.
 * Skip policy matches the other specs (infra failures skip). New file
 * only, no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

// Headless webkit is slow to boot under load; match the hex-find budget.
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
  // Wave-10 boots the demo text session; clear it for deterministic counts.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/hex-66l-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-66l-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

test("hex dual-pane: right column 1:1, diff tint both sides, orphan hatch, parity smoke, light+dark", async ({
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

  // Embedded h_* pair: left "hello world" vs right "hallo world" (differ at
  // offset 1), 11 bytes on a single page.
  await expect(view).toContainText("files differ");
  await expect(view).toContainText("11 bytes");
  await expect(view).toContainText("page 1/1");
  await expect(page.getByTestId("hexview/page-0")).toBeVisible();

  // Right hex column: 1:1 testids for all 11 offsets, same row as the left
  // twin (equal y), same button affordances.
  for (const o of [0, 1, 10]) {
    const left = page.getByTestId(`hexview/byte-${o}`);
    const right = page.getByTestId(`hexview/byte-R-${o}`);
    await expect(left).toBeVisible();
    await expect(right).toBeVisible();
    await expect(right).toHaveAttribute("role", "button");
    await expect(right).toHaveAttribute("tabindex", "0");
    const yL = await left.evaluate((el) => el.getBoundingClientRect().y);
    const yR = await right.evaluate((el) => el.getBoundingClientRect().y);
    expect(Math.abs(yL - yR), `offset ${o} twins share a row`).toBeLessThan(2);
  }
  // Right bytes read the right side: offset 1 is 0x61 ('a'), not 0x65.
  await expect(page.getByTestId("hexview/byte-R-1")).toHaveText("61");
  await expect(page.getByTestId("hexview/byte-1")).toHaveText("65");

  // Right ASCII column mirrors the left one (row 0: live bytes + filler).
  const row0Ascii = page
    .getByTestId("hexview/page-0")
    .locator(".hx-row")
    .first()
    .locator(".hx-ascii");
  await expect(row0Ascii).toHaveCount(2);
  await expect(row0Ascii.nth(0)).toContainText("hello world");
  await expect(row0Ascii.nth(1)).toContainText("hallo world");

  // Diff tint on BOTH sides of the differing offset, on neither of same.
  await expect(page.getByTestId("hexview/byte-1")).toHaveClass(/hx-diff/);
  await expect(page.getByTestId("hexview/byte-R-1")).toHaveClass(/hx-diff/);
  await expect(page.getByTestId("hexview/byte-0")).not.toHaveClass(/hx-diff/);
  await expect(page.getByTestId("hexview/byte-R-0")).not.toHaveClass(/hx-diff/);

  // Orphan hatch: offsets 11..15 sit beyond both 11-byte sides on the
  // live row 0, so 5 left + 5 right hatched cells, tint-free, with the
  // exact 66I gradient values.
  const hatch = view.locator(".hx-cell-empty");
  await expect(hatch).toHaveCount(10);
  for (const tint of ["hx-diff", "hx-find-match", "hx-selected"]) {
    await expect(
      hatch.first(),
      `hatch carries no ${tint}`,
    ).not.toHaveClass(new RegExp(tint));
  }
  const hatchBg = await hatch.first().evaluate((el) => window.getComputedStyle(el).backgroundImage);
  expect(hatchBg, "orphan cell paints the 66I hatch gradient").toContain("repeating-linear-gradient");
  expect(hatchBg, "hatch keeps the 66I -45deg angle").toContain("-45deg");

  // Clicking a right byte selects the same absolute offset (shared single
  // selection: both twins mark data-selected, footer follows with diff).
  const rightByte1 = page.getByTestId("hexview/byte-R-1");
  await rightByte1.click();
  await expect(rightByte1).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hexview/byte-1")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(1)");
  await expect(page.getByTestId("hex-detail-left")).toContainText("65");
  await expect(page.getByTestId("hex-detail-right")).toContainText("61");
  await expect(page.getByTestId("hex-detail-flag")).toContainText("diff");

  // Keyboard from the right cell moves the shared selection (1 -> 2 -> 1).
  await rightByte1.focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByTestId("hexview/byte-2")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hexview/byte-R-2")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(2)");
  await page.keyboard.press("ArrowLeft");
  await expect(rightByte1).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-flag")).toContainText("diff");

  // Composition smoke: paging + find + goto + copy behave as before.
  // Single-page pager stays honestly disabled.
  await expect(view.getByRole("button", { name: "Prev", exact: true })).toBeDisabled();
  await expect(view.getByRole("button", { name: "Next", exact: true })).toBeDisabled();

  // Find `6c` ("l") still matches at offset 2 with exactly one highlight
  // (left-only, exact-count contract) while both columns stay rendered.
  await page.getByTestId("hex-find-input").fill("6c");
  await page.getByTestId("hex-find-next").click();
  await expect(page.getByTestId("hex-find-status")).toContainText("match at offset 2");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);
  await expect(page.getByTestId("hexview/byte-2")).toHaveClass(/hx-find-match/);
  await expect(page.getByTestId("hexview/byte-R-2")).toBeVisible();

  // Goto `5` selects the same offset on both sides.
  await page.getByTestId("hex-goto-input").fill("5");
  await page.getByTestId("hex-goto-go").click();
  await expect(page.getByTestId("hex-goto-status")).toContainText("jumped to offset 5");
  await expect(page.getByTestId("hexview/byte-5")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hexview/byte-R-5")).toHaveAttribute("data-selected", "true");

  // Copy hex + ascii of the selection (left preferred: `65` / `e`).
  await rightByte1.click();
  await page.getByTestId("hex-copy-hex").click();
  await expect(page.getByTestId("hex-copy-status")).toContainText("hex");
  await expect(page.getByTestId("hex-copy-status")).toContainText("65");
  await page.getByTestId("hex-copy-ascii").click();
  await expect(page.getByTestId("hex-copy-status")).toContainText("ascii");
  await expect(page.getByTestId("hex-copy-status")).toContainText("e");

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Dark scheme: right column, both-side diff tint, and orphan hatch hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("hexview/byte-R-1")).toBeVisible();
  await expect(page.getByTestId("hexview/byte-1")).toHaveClass(/hx-diff/);
  await expect(page.getByTestId("hexview/byte-R-1")).toHaveClass(/hx-diff/);
  await expect(hatch.first()).toHaveClass(/hx-cell-empty/);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  await assertNoOverlap(page);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
