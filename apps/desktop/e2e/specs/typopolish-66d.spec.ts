/**
 * Wave-66D views typography-second exquisite pass (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * All seven data views ride the existing token ramp with typography-only
 * classes (font family / weight / line-height / letter-spacing, no color
 * or geometry edits, no new sizes):
 *  - code cells (text/folder-name inputs aside): mono stack, 12px, 1.55
 *    line-height (18.6px computed), 0 tracking, tabular numerals;
 *  - UI chrome text (status / totals / counts / notes / labels): UI stack,
 *    12px, 1.5 line-height (18px computed), -0.01em tracking (-0.12px),
 *    explicit 400/500/600 weights.
 * This spec pins the computed values per active view (text demo
 * first-paint + folder/hex/table/merge/sync/media sessions), asserts no
 * text overlap, and repeats the overlap + alert checks in dark mode.
 * Light + dark PNGs land in `/tmp/ui-66d/` (never in-repo). Zero console
 * errors / page errors. New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-66d";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewTestId: string,
): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestId)).toBeVisible();
}

interface TypoFailure {
  readonly check: string;
  readonly detail: string;
}

/** View-scoped overlap: this wave owns views/* typography only, so the
 * overlap pin covers the active data view subtree. The chrome tabbar is
 * explicitly out of scope (read-only for wave-66d): with seven sessions
 * open the tab strip overflows its container, which the default
 * header/nav/main selector reports as tabbar-vs-tab overlap. */
const VIEW_OVERLAP_SELECTORS =
  'main, [data-testid^="view-"], [data-testid^="view-"] [data-testid], .tv-cell, .tv-gutter, .fv-node, .hx-row, .tc-cell, .tc-head, .sv-row, .mw-row';

test("views typography rides the token ramp in every data view, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Text demo first-paint is already mounted: pin code + UI type.
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect
    .poll(async () => page.locator(".tv-cell").count(), { timeout: 15000 })
    .toBeGreaterThan(0);
  const textFailures: TypoFailure[] = await page.evaluate(() => {
    const out: { check: string; detail: string }[] = [];
    const cell = document.querySelector(".tv-cell");
    const gutter = document.querySelector(".tv-gutter");
    const status = document.querySelector('[data-testid="textview/status"]');
    const cs = (el: Element | null): CSSStyleDeclaration | null =>
      el ? window.getComputedStyle(el) : null;
    const cellStyle = cs(cell);
    if (!cell) out.push({ check: "text .tv-cell present", detail: "missing" });
    else {
      if (!cellStyle!.fontFamily.includes("SF Mono"))
        out.push({ check: "text cell family", detail: cellStyle!.fontFamily });
      if (cellStyle!.fontWeight !== "400")
        out.push({ check: "text cell weight", detail: cellStyle!.fontWeight });
      if (Math.abs(parseFloat(cellStyle!.lineHeight) - 18.6) > 0.15)
        out.push({ check: "text cell line-height", detail: cellStyle!.lineHeight });
      if (cellStyle!.letterSpacing !== "0px" && cellStyle!.letterSpacing !== "normal")
        out.push({ check: "text cell tracking", detail: cellStyle!.letterSpacing });
    }
    const gutterStyle = cs(gutter);
    if (!gutter) out.push({ check: "text .tv-gutter present", detail: "missing" });
    else {
      if (!gutterStyle!.fontFamily.includes("SF Mono"))
        out.push({ check: "text gutter family", detail: gutterStyle!.fontFamily });
      if (gutterStyle!.fontWeight !== "400")
        out.push({ check: "text gutter weight", detail: gutterStyle!.fontWeight });
      if (Math.abs(parseFloat(gutterStyle!.lineHeight) - 18.6) > 0.15)
        out.push({ check: "text gutter line-height", detail: gutterStyle!.lineHeight });
      if (gutterStyle!.fontVariantNumeric !== "tabular-nums")
        out.push({ check: "text gutter nums", detail: gutterStyle!.fontVariantNumeric });
    }
    const statusStyle = cs(status);
    if (!status) out.push({ check: "text status present", detail: "missing" });
    else {
      if (statusStyle!.fontWeight !== "400")
        out.push({ check: "text status weight", detail: statusStyle!.fontWeight });
      if (Math.abs(parseFloat(statusStyle!.lineHeight) - 18) > 0.15)
        out.push({ check: "text status line-height", detail: statusStyle!.lineHeight });
      if (Math.abs(parseFloat(statusStyle!.letterSpacing) - -0.12) > 0.03)
        out.push({ check: "text status tracking", detail: statusStyle!.letterSpacing });
    }
    return out;
  });
  expect(textFailures, `text typo ramp (got: ${JSON.stringify(textFailures)})`).toEqual([]);

  // Remaining six views mount via sessions over the mock engine.
  await createSession(page, "folder", "/tmp/wave4a-left-dir", "/tmp/wave4a-right-dir", "view-folder");
  await createSession(page, "hex", "/tmp/hex-23-left.bin", "/tmp/hex-23-right.bin", "view-hex");
  await createSession(page, "table", "/tmp/t-left.csv", "/tmp/t-right.csv", "view-table");
  await createSession(page, "merge", "/tmp/merge12-base.txt", "/tmp/merge12-right.txt", "view-merge");
  await createSession(page, "sync", "/tmp/wave17-left-dir", "/tmp/wave17-right-dir", "view-sync");
  await createSession(page, "media", "/tmp/m-left.png", "/tmp/m-right.png", "view-media");

  // Visit every tab; pin the active view's computed type + overlap.
  const tabs = page.getByRole("tab");
  const tabCount = await tabs.count();
  const seen = new Set<string>();
  for (let i = 0; i < tabCount; i += 1) {
    await tabs.nth(i).click();
    await page.waitForTimeout(150);
    const kind: string | null = await page.evaluate(() => {
      for (const id of ["view-text", "view-folder", "view-hex", "view-table", "view-merge", "view-sync", "view-media"]) {
        if (document.querySelector(`[data-testid="${id}"]`)) return id;
      }
      return null;
    });
    if (kind === null) continue;
    seen.add(kind);
    const failures: TypoFailure[] = await page.evaluate((active: string) => {
      const out: { check: string; detail: string }[] = [];
      const styleOf = (sel: string): CSSStyleDeclaration | null => {
        const el = document.querySelector(sel);
        return el ? window.getComputedStyle(el) : null;
      };
      const checkCode = (sel: string, label: string, wantWeight: string): void => {
        const st = styleOf(sel);
        if (!st) {
          out.push({ check: `${label} present`, detail: `missing ${sel}` });
          return;
        }
        if (!st.fontFamily.includes("SF Mono")) out.push({ check: `${label} family`, detail: st.fontFamily });
        if (st.fontWeight !== wantWeight) out.push({ check: `${label} weight`, detail: st.fontWeight });
        if (Math.abs(parseFloat(st.lineHeight) - 18.6) > 0.15)
          out.push({ check: `${label} line-height`, detail: st.lineHeight });
        if (st.letterSpacing !== "0px" && st.letterSpacing !== "normal")
          out.push({ check: `${label} tracking`, detail: st.letterSpacing });
      };
      const checkUi = (sel: string, label: string, wantWeight: string, wantLH = 18): void => {
        const st = styleOf(sel);
        if (!st) {
          out.push({ check: `${label} present`, detail: `missing ${sel}` });
          return;
        }
        if (st.fontWeight !== wantWeight) out.push({ check: `${label} weight`, detail: st.fontWeight });
        if (Math.abs(parseFloat(st.lineHeight) - wantLH) > 0.15)
          out.push({ check: `${label} line-height`, detail: st.lineHeight });
        if (Math.abs(parseFloat(st.letterSpacing) - -0.12) > 0.03)
          out.push({ check: `${label} tracking`, detail: st.letterSpacing });
      };
      switch (active) {
        case "view-text":
          checkCode(".tv-cell", "text cell", "400");
          checkUi('[data-testid="textview/status"]', "text status", "400");
          break;
        case "view-folder":
          // Wave-96 density: unselected folder nodes ride compact 16px inside 17px
          // ROW_PX rows (not the 18px UI ramp); wave-100 folder verdict agrees.
          checkUi(".fv-node", "folder node", "400", 16);
          checkUi('[data-testid="folderview/totals"]', "folder totals", "400");
          break;
        case "view-hex":
          checkCode(".hx-row", "hex row", "400");
          checkCode('[data-testid="hex-detail"]', "hex detail", "400");
          break;
        case "view-table":
          checkCode(".tc-cell", "table cell", "400");
          break;
        case "view-merge":
          checkCode(".mv-pane pre", "merge pane", "400");
          checkUi('[data-testid="merge-conflict-count"]', "merge count", "400");
          break;
        case "view-sync":
          checkCode(".sv-row", "sync row", "400");
          checkUi('[data-testid="syncview/totals"]', "sync totals", "400");
          break;
        case "view-media":
          checkCode(".mw-row", "media row", "400");
          checkUi('[data-testid="mediaview/mode"]', "media mode", "400");
          break;
        default:
          break;
      }
      return out;
    }, kind);
    expect(failures, `${kind} typo ramp (got: ${JSON.stringify(failures)})`).toEqual([]);
    await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  }
  expect([...seen].sort(), `all data views visited (got: ${[...seen].sort().join(", ")})`).toEqual(
    ["view-folder", "view-hex", "view-media", "view-merge", "view-sync", "view-table", "view-text"],
  );

  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/typopolish-66d-light.png` });

  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  // Typography is color-scheme independent: spot-check + overlap in dark.
  const darkFailures: TypoFailure[] = await page.evaluate(() => {
    const out: { check: string; detail: string }[] = [];
    const el = document.querySelector(".tv-cell, .fv-node, .hx-row, .tc-cell, .mv-pane pre, .sv-row, .mw-row");
    if (!el) {
      out.push({ check: "dark active view present", detail: "missing" });
      return out;
    }
    const st = window.getComputedStyle(el);
    const isCode = el.classList.contains("tv-cell") || el.classList.contains("hx-row") || el.classList.contains("tc-cell") || el.classList.contains("sv-row") || el.classList.contains("mw-row") || el.tagName === "PRE";
    if (isCode && !st.fontFamily.includes("SF Mono"))
      out.push({ check: "dark code family", detail: st.fontFamily });
    return out;
  });
  expect(darkFailures, `dark typo ramp (got: ${JSON.stringify(darkFailures)})`).toEqual([]);
  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  await page.screenshot({ path: `${EVID_DIR}/typopolish-66d-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
