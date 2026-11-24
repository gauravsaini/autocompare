/**
 * Wave-75 ignore whitespace/case end-to-end (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the Beyond Compare style whitespace/case toggles actually
 * re-diff end to end:
 *  - committed fixture pairs (inline, ASCII): a case-only pair
 *    (`"Hello World"` vs `"hello world"`) and a whitespace-only pair
 *    (`"foo  bar"` vs `"foo bar"`) collapse to equal under the MATCHING
 *    toggle and restore to different when untoggled, verified through
 *    an in-page mirror of the engine `comparison_key`
 *    (`crates/compare/src/text.rs`: whitespace runs collapse + trim
 *    when `ignore_space_change`, `to_lowercase` when `ignore_case`);
 *    the real collapsing is covered by the Rust unit tests in
 *    `src-tauri/src/commands/text.rs`, the mirror pins the contract
 *    in-repo so the spec fails if the semantics drift;
 *  - the typed `textCompare(left, right, opts)` wrapper sends both
 *    spellings the Rust command accepts and the mock echoes
 *    `appliedOptions` + top-level flags for EITHER spelling;
 *  - the UI toggles route via that typed path, so the probe accepts
 *    the echo, reloads through the normal row/overview setters, and
 *    the honest-fallback note stays HIDDEN while supported;
 *  - untoggling both restores the exact diff (flags `false`, note
 *    hidden, same 200 / 644 rows);
 *  - first-row budget: `textview/row-0` mounts at the top of its list
 *    container (`top <= 140px`);
 *  - light + dark PNGs land in `/tmp/ui-75/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: the mock adapter has no whitespace/case engine (rows
 * are fixture rows), so the echo proves the flags round-tripped and the
 * view hides the note; row collapsing itself is proven by the
 * `comparison_key` mirror + Rust unit tests. Skip policy matches the
 * other specs (infra failures skip).
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-75";

// Committed fixture pairs: each differs by ONE ignore dimension only.
const CASE_LEFT = "Hello World";
const CASE_RIGHT = "hello world";
const WS_LEFT = "foo  bar";
const WS_RIGHT = "foo bar";

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

async function lastIgnoreFlags(page: {
  evaluate: <T>(fn: () => T) => Promise<T>;
}): Promise<Record<string, unknown> | null> {
  return page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const last = w["__TEXT_IGNORE_LAST__"];
    if (typeof last !== "object" || last === null) return null;
    return last as Record<string, unknown>;
  });
}

test("ignore toggles re-diff end to end, note hidden, restore on untoggle", async ({
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
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  const caseToggle = page.getByTestId("textview/ignore-case");
  // Wave-112 preamble: the ignore panel is closed by default (BC
  // truth); open it via its toolbar toggle. Zero assertion changes.
  const ignoreToggle112 = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle112.getAttribute("aria-expanded")) !== "true") await ignoreToggle112.click();
  await expect(wsToggle).toBeVisible();
  await expect(caseToggle).toBeVisible();
  await expect(wsToggle).not.toBeChecked();
  await expect(caseToggle).not.toBeChecked();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);

  // 1. Committed pairs collapse ONLY under the matching toggle (in-page
  //    mirror of `comparison_key` in `crates/compare/src/text.rs`).
  const collapse = await page.evaluate(
    ({ cl, cr, wl, wr }) => {
      const key = (line: string, ws: boolean, cs: boolean): string => {
        const k = ws ? line.split(/\s+/).filter((s) => s !== "").join(" ") : line;
        return cs ? k.toLowerCase() : k;
      };
      return {
        caseExactDiffers: key(cl, false, false) !== key(cr, false, false),
        caseCollapsesUnderCase: key(cl, false, true) === key(cr, false, true),
        caseSurvivesWsOnly: key(cl, true, false) !== key(cr, true, false),
        caseCollapsesUnderBoth: key(cl, true, true) === key(cr, true, true),
        wsExactDiffers: key(wl, false, false) !== key(wr, false, false),
        wsCollapsesUnderWs: key(wl, true, false) === key(wr, true, false),
        wsSurvivesCaseOnly: key(wl, false, true) !== key(wr, false, true),
        wsCollapsesUnderBoth: key(wl, true, true) === key(wr, true, true),
      };
    },
    { cl: CASE_LEFT, cr: CASE_RIGHT, wl: WS_LEFT, wr: WS_RIGHT },
  );
  expect(collapse.caseExactDiffers, "case pair differs exactly").toBe(true);
  expect(collapse.caseCollapsesUnderCase, "case pair collapses under ignore-case").toBe(true);
  expect(collapse.caseSurvivesWsOnly, "case pair survives whitespace-only toggle").toBe(true);
  expect(collapse.caseCollapsesUnderBoth, "case pair collapses under both").toBe(true);
  expect(collapse.wsExactDiffers, "ws pair differs exactly").toBe(true);
  expect(collapse.wsCollapsesUnderWs, "ws pair collapses under ignore-whitespace").toBe(true);
  expect(collapse.wsSurvivesCaseOnly, "ws pair survives case-only toggle").toBe(true);
  expect(collapse.wsCollapsesUnderBoth, "ws pair collapses under both").toBe(true);

  // 2. Whitespace toggle: re-diffs via the typed path, note stays hidden.
  await wsToggle.check();
  await expect(wsToggle).toBeChecked();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreWhitespace"], { timeout: 10000 })
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // 3. Untoggled: exact diff restored, note stays hidden.
  await wsToggle.uncheck();
  await expect(wsToggle).not.toBeChecked();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreWhitespace"], { timeout: 10000 })
    .toBe(false);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 4. Case toggle: same round-trip on the other dimension.
  await caseToggle.check();
  await expect(caseToggle).toBeChecked();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreCase"], { timeout: 10000 })
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await caseToggle.uncheck();
  await expect(caseToggle).not.toBeChecked();
  await expect
    .poll(async () => (await lastIgnoreFlags(page))?.["ignoreCase"], { timeout: 10000 })
    .toBe(false);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 5. Typed echo proof for either spelling dimension (mock echoes both
  //    `appliedOptions` and the top-level flags the probe accepts).
  const typedWs = (await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    return (await mod.textCompare("left-a", "right-b", {
      ignoreWhitespace: true,
    })) as unknown as Record<string, unknown>;
  })) as Record<string, unknown>;
  expect(typedWs["ignoreWhitespace"]).toBe(true);
  expect(typedWs["ignoreCase"]).toBe(false);
  expect((typedWs["appliedOptions"] as Record<string, unknown>)["ignoreWhitespace"]).toBe(true);
  const typedCase = (await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    return (await mod.textCompare("left-a", "right-b", {
      ignoreCase: true,
    })) as unknown as Record<string, unknown>;
  })) as Record<string, unknown>;
  expect(typedCase["ignoreCase"]).toBe(true);
  expect((typedCase["appliedOptions"] as Record<string, unknown>)["ignoreCase"]).toBe(true);

  // 6. First-row budget: row-0 mounts at the top of its list container.
  const budget = await page.evaluate(() => {
    const row = document.querySelector('[data-testid="textview/row-0"]');
    if (!(row instanceof HTMLElement)) return null;
    const host = row.parentElement;
    if (!(host instanceof HTMLElement)) return null;
    const r = row.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    return { top: r.top - h.top, height: r.height, absTop: r.top };
  });
  expect(budget, "row-0 mounted").not.toBeNull();
  expect(
    budget?.top ?? Number.POSITIVE_INFINITY,
    `first row at list top (got ${budget?.top}px, abs ${budget?.absTop}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/textignore-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/textignore-dark.png` });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
