/**
 * Wave-66K folder one-sided row BC-parity proof (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> FolderView) over the mock folder engine (`dumped.json`:
 * 28 leftOnly + 28 rightOnly nodes with null missing sides, served with
 * real paging semantics by `src/ipc/client.ts`) and proves the 66I/66J
 * orphan idiom fanned out to folder one-sided rows:
 *  - missing-side filler: every leftOnly/rightOnly row mounts
 *    `fv-cell-empty` on the row div (the exact 66I
 *    `repeating-linear-gradient(-45deg, ...)` hatch over `var(--ac-bg)`)
 *    plus `data-side="left-only"` / `"right-only"`;
 *  - present-side cue: the StatusIcon arrows for one-sided rows ONLY
 *    restyle to the 66I idiom — 12px golden-amber (`--ac-tab-dirty`)
 *    chevrons (ChevronRight for leftOnly pointing outward, ChevronLeft
 *    mirrored for rightOnly) with aria-labels `"left only"` /
 *    `"right only"` kept; all other statuses stay byte-identical
 *    (same Check 14, different ArrowLeftRight 14);
 *  - bracket rail: folder rows are single-line, so one-sided rows
 *    expose `data-bracket="single"` (no `::before` rail paints) for
 *    spec symmetry with 66I/66J, while the `multi` rail rule exists in
 *    the stylesheet (1px `--ac-fg-muted` ink, `pointer-events: none`);
 *    two-sided rows mount NO hatch, NO `data-side`, NO `data-bracket`;
 *  - row identity untouched: `folder-row-<i>` testids, `data-status`
 *    and `data-path` behave exactly as before;
 *  - one composition smoke (filter + expand/collapse) proves the
 *    existing behavior is intact (filters/collapse/detail/copy logic
 *    untouched);
 *  - light + dark DOM checks (no PNGs committed), assertNoOverlap,
 *    zero console errors / page errors throughout.
 *
 * Mock honesty: everything above reads the DOM over the mock nodes (no
 * new IPC: the totals label is byte-identical throughout). Skip policy
 * matches the other specs (infra failures skip). New file only, no
 * existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

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

const totalsText = (page: Page): Promise<string> =>
  page
    .getByTestId("folderview/totals")
    .textContent()
    .then((t) => (t ?? "").trim());

/** All currently rendered rows of one orphan side. */
function orphanRows(page: Page, status: "leftOnly" | "rightOnly") {
  return page.locator(`[data-testid^="folder-row-"][data-status="${status}"]`);
}

/** The 66I-idiom chevron svg of an orphan row (aria-label kept). */
function orphanChevron(page: Page, rowIndex: number, label: "left only" | "right only") {
  return page.getByTestId(`folder-row-${rowIndex}`).locator(`svg[aria-label="${label}"]`);
}

async function rowIndexByPath(page: Page, relPath: string): Promise<number> {
  const testid = await page
    .locator(`[data-testid^="folder-row-"][data-path="${relPath}"]`)
    .getAttribute("data-testid");
  return Number((testid ?? "").replace("folder-row-", ""));
}

test("folder one-sided rows: orphan hatch, amber chevrons, single bracket, behavior intact", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createFolderSession(page);
  const view = page.getByTestId("view-folder");

  // First window: 50 loaded entries, all one-sided (28 leftOnly + 22
  // rightOnly per the mock engine order).
  await expect(page.getByTestId("folderview/totals")).toContainText("50 / 590 entries");
  const rows = page.locator('[data-testid^="folder-row-"]');
  await expect.poll(async () => rows.count(), { timeout: 10000 }).toBe(50);

  // Filter leftOnly: only left-only rows, each with the hatch class,
  // data-side, single bracket and the 12px amber outward chevron.
  await expect.poll(async () => orphanRows(page, "leftOnly").count()).toBe(28);
  await expect.poll(async () => orphanRows(page, "rightOnly").count()).toBe(22);
  for (let n = 0; n < 28; n += 1) {
    const row = page.getByTestId(`folder-row-${n}`);
    await expect(row).toHaveAttribute("data-status", "leftOnly");
    await expect(row).toHaveClass(/fv-cell-empty/);
    await expect(row).toHaveAttribute("data-side", "left-only");
    await expect(row).toHaveAttribute("data-bracket", "single");
    const chevron = orphanChevron(page, n, "left only");
    await expect(chevron).toBeVisible();
    const box = await chevron.evaluate((el) => el.getBoundingClientRect().toJSON());
    expect(Math.round(box.width), `folder-row-${n} chevron is 12px`).toBe(12);
    expect(Math.round(box.height), `folder-row-${n} chevron is 12px`).toBe(12);
    const color = await chevron.evaluate((el) => window.getComputedStyle(el).color);
    expect(color, `folder-row-${n} chevron keeps golden-amber --ac-tab-dirty ink`).toBe(
      "rgb(255, 159, 10)",
    );
    const path = await chevron.locator("path").getAttribute("d");
    expect(path, `folder-row-${n} leftOnly chevron points outward (right)`).toContain("m9 18");
  }
  // Filter rightOnly (first window holds 22 of the 28): mirrored idiom.
  for (let n = 28; n < 50; n += 1) {
    const row = page.getByTestId(`folder-row-${n}`);
    await expect(row).toHaveAttribute("data-status", "rightOnly");
    await expect(row).toHaveClass(/fv-cell-empty/);
    await expect(row).toHaveAttribute("data-side", "right-only");
    await expect(row).toHaveAttribute("data-bracket", "single");
    const chevron = orphanChevron(page, n, "right only");
    await expect(chevron).toBeVisible();
    const box = await chevron.evaluate((el) => el.getBoundingClientRect().toJSON());
    expect(Math.round(box.width), `folder-row-${n} chevron is 12px`).toBe(12);
    expect(Math.round(box.height), `folder-row-${n} chevron is 12px`).toBe(12);
    const color = await chevron.evaluate((el) => window.getComputedStyle(el).color);
    expect(color, `folder-row-${n} chevron keeps golden-amber --ac-tab-dirty ink`).toBe(
      "rgb(255, 159, 10)",
    );
    const path = await chevron.locator("path").getAttribute("d");
    expect(path, `folder-row-${n} rightOnly chevron points outward (left)`).toContain("m15 18");
  }

  // The hatch paints the exact 66I gradient on a sample of each side.
  for (const n of [0, 28]) {
    const hatch = await page
      .getByTestId(`folder-row-${n}`)
      .evaluate((el) => window.getComputedStyle(el).backgroundImage);
    expect(hatch, `folder-row-${n} paints the 66I hatch gradient`).toContain(
      "repeating-linear-gradient",
    );
    expect(hatch, `folder-row-${n} hatch keeps the 66I -45deg angle`).toContain("-45deg");
  }
  // Single-line rows mount no rail: no `::before` on a `single` bracket.
  const singleBefore = await page
    .getByTestId("folder-row-0")
    .evaluate((el) => window.getComputedStyle(el, "::before").content);
  expect(singleBefore, "single-bracket row mounts no rail").toBe("none");
  // ... while the multi-line branch exists as a pure-CSS rule, exactly
  // like the 66J card rail (shared bracket-rail idiom in the stylesheet).
  const railRules = await page.evaluate(() => {
    const hits: string[] = [];
    for (const sheet of Array.from(document.styleSheets)) {
      let rules: CSSRuleList | null = null;
      try {
        rules = sheet.cssRules;
      } catch {
        continue;
      }
      if (rules === null) continue;
      for (const rule of Array.from(rules)) {
        if (
          rule instanceof CSSStyleRule &&
          rule.selectorText.includes('.fv-rail[data-bracket="multi"]::before')
        ) {
          hits.push(rule.cssText);
        }
      }
    }
    return hits;
  });
  expect(railRules.length, "multi bracket rail rule exists").toBeGreaterThan(0);
  const railCss = railRules.join("\n");
  expect(railCss, "rail is a 1px stem").toContain("1px");
  // Wave-105 renamed the muted rail ink to `--ac-rail-accent` (identical hex
  // #6e6e73/#a1a1a6, pinned in parity-colors-105.spec.ts) — accept the alias.
  expect(railCss, "rail reuses the existing muted ink").toContain("--ac-rail-accent");
  expect(railCss, "rail never intercepts row clicks").toContain("pointer-events: none");

  // Second window completes the 28 rightOnly nodes and adds two-sided
  // rows (same/different/newerRight).
  const loadedBefore = await totalsText(page);
  await page.getByTestId("folderview/load-more").click();
  await expect.poll(async () => totalsText(page), { timeout: 10000 }).not.toBe(loadedBefore);
  const loadedTotals = await totalsText(page);
  expect(loadedTotals).toBe("100 / 590 entries");
  await expect.poll(async () => orphanRows(page, "rightOnly").count()).toBe(28);
  // Every rightOnly row (all 28 now) carries the mirrored idiom.
  for (let k = 0; k < 28; k += 1) {
    const row = orphanRows(page, "rightOnly").nth(k);
    await expect(row).toHaveClass(/fv-cell-empty/);
    await expect(row).toHaveAttribute("data-side", "right-only");
    await expect(row).toHaveAttribute("data-bracket", "single");
  }

  // Two-sided rows carry NO hatch and NO bracket/side markers, and their
  // status icons stay byte-identical (Check 14 / ArrowLeftRight 14).
  const sameIdx = await rowIndexByPath(page, "d01/f00.txt");
  const sameRow = page.getByTestId(`folder-row-${sameIdx}`);
  await expect(sameRow).toHaveAttribute("data-status", "same");
  await expect(sameRow).not.toHaveClass(/fv-cell-empty/);
  await expect(sameRow).not.toHaveAttribute("data-bracket");
  await expect(sameRow).not.toHaveAttribute("data-side");
  const sameIcon = sameRow.locator('svg[aria-label="same"]');
  await expect(sameIcon).toBeVisible();
  const sameBox = await sameIcon.evaluate((el) => el.getBoundingClientRect().toJSON());
  expect(Math.round(sameBox.width), "same icon stays 14px").toBe(14);
  expect(Math.round(sameBox.height), "same icon stays 14px").toBe(14);

  const diffIdx = await rowIndexByPath(page, "d02/f02.txt");
  const diffRow = page.getByTestId(`folder-row-${diffIdx}`);
  await expect(diffRow).toHaveAttribute("data-status", "different");
  await expect(diffRow).not.toHaveClass(/fv-cell-empty/);
  await expect(diffRow).not.toHaveAttribute("data-bracket");
  await expect(diffRow).not.toHaveAttribute("data-side");
  const diffIcon = diffRow.locator('svg[aria-label="different"]');
  await expect(diffIcon).toBeVisible();
  const diffBox = await diffIcon.evaluate((el) => el.getBoundingClientRect().toJSON());
  expect(Math.round(diffBox.width), "different icon stays 14px").toBe(14);
  expect(Math.round(diffBox.height), "different icon stays 14px").toBe(14);

  // Dark scheme: hatch, brackets, side markers and amber chevrons hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(page.getByTestId("folder-row-0")).toHaveClass(/fv-cell-empty/);
  await expect(page.getByTestId("folder-row-0")).toHaveAttribute("data-side", "left-only");
  await expect(page.getByTestId("folder-row-0")).toHaveAttribute("data-bracket", "single");
  await expect(sameRow).not.toHaveClass(/fv-cell-empty/);
  await expect(sameRow).not.toHaveAttribute("data-bracket");
  const darkChevron = await orphanChevron(page, 0, "left only").evaluate(
    (el) => window.getComputedStyle(el).color,
  );
  expect(darkChevron, "chevron stays golden-amber in dark").toBe("rgb(255, 159, 10)");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  // Composition smoke: expand/collapse + filters behave exactly as
  // before (totals label unchanged => no refetch, client-side only).
  const collapse0 = page.getByTestId("folder-collapse-0");
  await expect(collapse0).toBeVisible();
  const descendant = page.locator('[data-testid^="folder-row-"][data-path="d00/f00.txt"]');
  await expect(descendant).toHaveCount(1);
  const beforeCollapse = await rows.count();
  await collapse0.click();
  await expect(descendant).toHaveCount(0);
  expect(await rows.count(), "collapsing d00 hides its descendants").toBeLessThan(beforeCollapse);
  await collapse0.click();
  await expect(descendant).toHaveCount(1);
  expect(await rows.count(), "expanding d00 restores its descendants").toBe(beforeCollapse);

  const allBtn = page.getByTestId("folder-filter-all");
  const diffsBtn = page.getByTestId("folder-filter-diffs");
  const sameBtn = page.getByTestId("folder-filter-same");
  await diffsBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const diffStatuses: string[] = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
  for (const s of diffStatuses) {
    expect(s, "diffs filter hides every same row").not.toBe("same");
  }
  expect(await totalsText(page), "filtering never refetches").toBe(loadedTotals);
  await sameBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBeGreaterThan(0);
  const sameStatuses: string[] = await page.$$eval('[data-testid^="folder-row-"]', (nodes) =>
    nodes.map((n) => n.getAttribute("data-status") ?? ""),
  );
  for (const s of sameStatuses) {
    expect(s, "same filter shows only same rows").toBe("same");
  }
  await allBtn.click();
  await expect.poll(async () => rows.count(), { timeout: 5000 }).toBe(100);

  await expect(view).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  await assertNoOverlap(page);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
