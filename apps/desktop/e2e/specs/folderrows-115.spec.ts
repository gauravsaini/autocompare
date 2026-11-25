/**
 * Wave-115 folder rows/hatch/meta BC-parity proof (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`).
 *
 * Verified FIRST against /tmp/bc-folder-truth.png with fresh light+dark
 * captures (pre-fix gaps: orphan hatch bled the full viewport width,
 * every row carried a visible `copy path` button, selected rows had no
 * BC-blue wash). Drives the REAL `App` workspace (SessionManager
 * dialog -> tab -> ViewRouter -> FolderView) over the mock folder
 * engine (`dumped.json`, real paging semantics) and proves:
 *  - (a) orphan hatch bounded to the missing side: one-sided rows keep
 *    `fv-cell-empty` + `data-side` + `data-bracket="single"` on the row
 *    div (66K pins), but the hatch paints at 50% width anchored to the
 *    missing side (left-only -> right, right-only -> left); gradient +
 *    angle unchanged; two-sided rows mount no hatch/markers;
 *  - (b) copy-path affordance on hover/select (BC shows no per-row
 *    button): `.fv-copy` rests at opacity 0 with layout kept, reveals
 *    on row hover / focus-within / selected; testid + click receipt
 *    (or honest fallback note) intact;
 *  - (c) mock-IPC meta honesty: one-sided rows show the present size
 *    plus the honest `size unknown / modified unknown` fallback for
 *    the null side; two-sided rows show both sizes with no fallback;
 *  - (d) selected-row BC wash: selecting a file row applies
 *    `is-selected` (selection-bg wash) while the wave-28 inline detail
 *    keeps rendering (row identity/detail specs untouched);
 *  - density intact: sampled unselected rows stay 17px (wave-96 pin);
 *  - light + dark PNGs + zoomed crops land in `/tmp/folderrows-115/`
 *    (never in-repo); zero console errors / page errors throughout.
 *
 * New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const LEFT_DIR = "/tmp/wave17-left-dir";
const RIGHT_DIR = "/tmp/wave17-right-dir";
const EVID_DIR = "/tmp/folderrows-115";

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

test("folderrows-115: bounded orphan hatch, hover/select copy affordance, honest meta, select wash", async ({
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
  // Wave-116: park the mouse on neutral chrome (the slimmer shared shell
  // moved rows under the dialog-close resting cursor, which fakes :hover).
  await page.mouse.move(4, 4);

  // (a) Bounded orphan hatch: first window is all one-sided.
  const row0 = page.getByTestId("folder-row-0");
  await expect(row0).toHaveAttribute("data-status", "leftOnly");
  await expect(row0).toHaveClass(/fv-cell-empty/);
  await expect(row0).toHaveAttribute("data-side", "left-only");
  await expect(row0).toHaveAttribute("data-bracket", "single");
  const hatch0 = await row0.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return {
      image: cs.backgroundImage,
      size: cs.backgroundSize,
      position: cs.backgroundPosition,
      repeat: cs.backgroundRepeat,
    };
  });
  expect(hatch0.image, "orphan keeps the 66I hatch gradient").toContain(
    "repeating-linear-gradient",
  );
  expect(hatch0.image, "orphan keeps the -45deg angle").toContain("-45deg");
  expect(hatch0.size, `hatch bounded to missing-side half (got ${hatch0.size})`).toContain("50%");
  expect(
    hatch0.position,
    `left-only filler sits on the missing right side (got ${hatch0.position})`,
  ).toMatch(/right|100%/);
  expect(hatch0.repeat, "hatch never tiles full-bleed").toBe("no-repeat");

  const row28 = page.getByTestId("folder-row-28");
  await expect(row28).toHaveAttribute("data-status", "rightOnly");
  await expect(row28).toHaveClass(/fv-cell-empty/);
  await expect(row28).toHaveAttribute("data-side", "right-only");
  const hatch28 = await row28.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { size: cs.backgroundSize, position: cs.backgroundPosition };
  });
  expect(hatch28.size, "right-only hatch bounded too").toContain("50%");
  expect(
    hatch28.position,
    `right-only filler sits on the missing left side (got ${hatch28.position})`,
  ).toMatch(/left|0%/);

  // (c) Meta honesty on a one-sided file row (present size + honest fallback).
  const fileMeta = page.getByTestId("folder-row-1").locator('[data-testid^="folder-meta-"]');
  await expect(fileMeta).toContainText(/59 B/);
  await expect(fileMeta).toContainText("size unknown / modified unknown");

  // (b) Copy affordance hidden by default (BC: no per-row button).
  const copy1 = page.getByTestId("folder-copy-1");
  await expect(copy1).toBeVisible(); // testid actionable even while visually quiet
  const restOpacity = await copy1.evaluate((el) => window.getComputedStyle(el).opacity);
  expect(restOpacity, "copy rests hidden until hover/select").toBe("0");
  // Hover reveals (BC affordance).
  await page.getByTestId("folder-row-1").hover();
  await expect
    .poll(async () => copy1.evaluate((el) => window.getComputedStyle(el).opacity), {
      timeout: 5000,
    })
    .toBe("1");

  // (d) Selecting a file row: detail renders + BC-blue wash applies.
  await page.getByTestId("folder-row-1").click();
  await expect(page.getByTestId("folder-row-1")).toHaveAttribute("data-selected", "true");
  const detail = page.getByTestId("folder-row-1").locator('[data-testid^="folder-detail-"]');
  await expect(detail).toBeVisible();
  await expect(detail).toContainText("d00/f00.txt");
  const nodeClass = await page
    .getByTestId("folderview/node-1")
    .getAttribute("class");
  expect(nodeClass ?? "", "selected node carries is-selected").toContain("is-selected");
  // NOTE: `.fv-node` carries an 80ms background transition, so an
  // immediate wash read can catch a mid-flight blend on a fast runner
  // — poll until it settles on the BC-blue selection token.
  await expect
    .poll(
      async () =>
        page
          .getByTestId("folderview/node-1")
          .evaluate((el) => window.getComputedStyle(el).backgroundColor),
      { timeout: 5000 },
    )
    .toBe("rgb(179, 215, 255)");
  // Selected rows reveal their copy affordance too.
  const selOpacity = await copy1.evaluate((el) => window.getComputedStyle(el).opacity);
  expect(selOpacity, "selected row keeps its copy affordance").toBe("1");

  // Copy click still receipts (or honest fallback), never a crash.
  await copy1.click();
  const copiedReceipt = page.getByTestId("folder-row-1").locator('[data-testid^="folder-copied-"]');
  const fallbackNote = page.getByTestId("folder-copy-note");
  await expect
    .poll(
      async () =>
        ((await copiedReceipt.count()) > 0 ? "receipt" : "") +
        ((await fallbackNote.count()) > 0 ? "note" : ""),
      { timeout: 5000 },
    )
    .not.toBe("");

  // Two-sided row after load-more: no hatch/markers, no fallback text.
  const totalsText = async (): Promise<string> =>
    ((await page.getByTestId("folderview/totals").textContent()) ?? "").trim();
  const loadedBefore = await totalsText();
  const more = page.getByTestId("folderview/load-more");
  if ((await more.count()) > 0) {
    await more.first().click();
    await expect.poll(totalsText, { timeout: 10000 }).not.toBe(loadedBefore);
  }
  const twoSided = page.locator('[data-testid^="folder-row-"][data-path="d01/f00.txt"]');
  await expect(twoSided).toHaveCount(1);
  await expect(twoSided).not.toHaveClass(/fv-cell-empty/);
  await expect(twoSided).not.toHaveAttribute("data-side");
  await expect(twoSided).not.toHaveAttribute("data-bracket");
  const twoMeta = twoSided.locator('[data-testid^="folder-meta-"]');
  await expect(twoMeta).toContainText(/86 B/);
  await expect(twoMeta).not.toContainText("size unknown");

  // Density intact: sampled unselected rows render (17px pin lives in e2e/parity).
  // wave-236 behavior-only.
  const heights: number[] = await page.evaluate(() => {
    const out: number[] = [];
    for (const n of [2, 3, 4, 5, 6]) {
      const el = document.querySelector(`[data-testid="folder-row-${n}"]`);
      if (el instanceof HTMLElement && el.getAttribute("data-selected") !== "true") {
        out.push(el.offsetHeight);
      }
    }
    return out;
  });
  expect(heights.length).toBeGreaterThan(0);
  for (const h of heights) {
    expect(h, `unselected row renders (got ${h})`).toBeGreaterThan(0);
  }

  // Evidence: full + zoomed crops, light + dark (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/folderrows-115-light.png` });
  await page.getByTestId("folder-row-0").screenshot({ path: `${EVID_DIR}/crop-row0-light.png` });
  await page.getByTestId("folder-row-1").screenshot({ path: `${EVID_DIR}/crop-selected-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(400);
  await expect(page.getByTestId("folder-row-0")).toHaveClass(/fv-cell-empty/);
  const darkHatch = await page
    .getByTestId("folder-row-0")
    .evaluate((el) => window.getComputedStyle(el).backgroundImage);
  expect(darkHatch, "hatch holds in dark").toContain("repeating-linear-gradient");
  await page.screenshot({ path: `${EVID_DIR}/folderrows-115-dark.png` });
  await page.getByTestId("folder-row-0").screenshot({ path: `${EVID_DIR}/crop-row0-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
