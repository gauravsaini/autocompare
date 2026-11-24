/**
 * Wave-66J merge conflict edge-gutter BC parity proof (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict merge
 * pair and proves the wave-66I idiom fanned out to merge conflict cards:
 *  - per-card take buttons keep their row BUT restyle to the 66I idiom:
 *    `merge-take-left-0` / `merge-take-right-0` render crisp 12px
 *    golden-amber chevrons (`--ac-tab-dirty`, borderless + transparent,
 *    hover highlight only) next to their text labels, and choosing still
 *    flips status exactly as before (`aria-pressed` + Lucide check icon,
 *    preview resolves, take-all composes);
 *  - section bracket: the card article exposes
 *    `data-bracket="single"` (the mock conflict is all-single-line:
 *    base 0 lines, left/right 1 line each) with NO `::before` rail, and
 *    the `data-bracket="multi"` rail rule exists in the stylesheet (1px
 *    `--ac-fg-muted` ink, `pointer-events: none`, full card height) for
 *    multi-line conflicts;
 *  - orphan hatching: the empty base pane (`(no lines)`) carries
 *    `mv-cell-empty` (the exact 66I `repeating-linear-gradient`
 *    hatch) while the live left/right panes carry no hatch class;
 *  - take-both + order + preview + apply untouched: one composition
 *    smoke (take-both L->R, order toggle R->L, take-all-right, find
 *    composes, apply receipt) proves the rest of the card is unchanged;
 *  - light + dark DOM checks (no PNGs committed), assertNoOverlap,
 *    zero console errors / page errors throughout.
 *
 * Mock honesty: under `VITE_MOCK_IPC=1` the view always plans the
 * embedded single-conflict fixture (id 0, base `[]`, left `["left"]`,
 * right `["right"]`), so only a single-line card ever renders here --
 * the `multi` branch is proved via its stylesheet rule, not a rendered
 * card. Everything else reads the DOM (no new IPC). Skip policy matches
 * the other specs (infra failures skip). New file only, no existing
 * spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-66j";
const OUT_FILE = `${EVID_DIR}/merge-gutter-out.txt`;

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createMergeSession(page: Page): Promise<void> {
  // Wave-10 boots the demo text session; clear it for deterministic counts.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/merge12-base.txt");
  await page.getByTestId("session/right-path").fill("/tmp/merge12-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-merge")).toBeVisible();
}

test("merge edge gutters: chevron take buttons, bracket rail, orphan hatch, choose parity", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMergeSession(page);
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");
  const preview = page.getByTestId("merge-preview");

  const card = page.getByTestId("mergeview/conflict-0");
  await expect(card).toBeVisible();

  // Section bracket: the mock conflict is all-single-line (base 0,
  // left/right 1), so the card reads `single` ...
  await expect(card).toHaveAttribute("data-bracket", "single");
  // ... with no rail painted (no `::before` at all).
  const beforeContent = await card.evaluate((el) => window.getComputedStyle(el, "::before").content);
  expect(beforeContent, "single-line card mounts no bracket rail").toBe("none");
  // ... while the multi-line branch exists as a pure-CSS rule: left-edge
  // 1px rail in the existing muted ink that never intercepts clicks.
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
          rule.selectorText.includes('.mv-rail[data-bracket="multi"]::before')
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
  expect(railCss, "rail never intercepts card clicks").toContain("pointer-events: none");

  // Take-left / take-right keep testids + labels but render the 66I
  // idiom: 12px golden-amber chevrons, no button box (borderless +
  // transparent, hover highlight only).
  await expect(page.getByTestId("merge-take-left-0")).toContainText("Take left");
  await expect(page.getByTestId("merge-take-right-0")).toContainText("Take right");
  for (const testid of ["merge-take-left-0", "merge-take-right-0"]) {
    const btn = page.getByTestId(testid);
    await expect(btn).toBeVisible();
    const svgBox = await btn.locator("svg").evaluate((el) => el.getBoundingClientRect().toJSON());
    expect(Math.round(svgBox.width), `${testid} arrow svg is 12px`).toBe(12);
    expect(Math.round(svgBox.height), `${testid} arrow svg is 12px`).toBe(12);
    const style = await btn.evaluate((el) => {
      const cs = window.getComputedStyle(el);
      return { color: cs.color, borderTopWidth: cs.borderTopWidth, backgroundColor: cs.backgroundColor };
    });
    expect(style.color, `${testid} golden-amber --ac-tab-dirty ink`).toBe("rgb(255, 159, 10)");
    expect(style.borderTopWidth, `${testid} no button box border`).toBe("0px");
    expect(style.backgroundColor, `${testid} no button box fill`).toBe("rgba(0, 0, 0, 0)");
  }

  // Choosing still flips status exactly as before (take-all pattern).
  await page.getByTestId("merge-take-left-0").click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("merge-take-left-0").locator("svg")).toHaveCount(2);
  await expect(preview).toHaveText("base\nleft");
  await page.getByTestId("merge-take-right-0").click();
  await expect(page.getByTestId("merge-take-right-0")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("merge-take-right-0").locator("svg")).toHaveCount(2);
  await expect(preview).toHaveText("base\nright");

  // Orphan hatching: the empty base pane shows `(no lines)` under the
  // hatch class; the live left/right panes carry no hatch class.
  const panes = card.locator("pre");
  await expect(panes).toHaveCount(3);
  await expect(panes.nth(0)).toHaveText("left");
  await expect(panes.nth(1)).toHaveText("(no lines)");
  await expect(panes.nth(2)).toHaveText("right");
  await expect(panes.nth(1)).toHaveClass(/mv-cell-empty/);
  await expect(panes.nth(0)).not.toHaveClass(/mv-cell-empty/);
  await expect(panes.nth(2)).not.toHaveClass(/mv-cell-empty/);
  const hatch = await panes.nth(1).evaluate((el) => window.getComputedStyle(el).backgroundImage);
  expect(hatch, "empty pane paints the 66I hatch gradient").toContain("repeating-linear-gradient");
  expect(hatch, "hatch keeps the 66I -45deg angle").toContain("-45deg");

  // Take-both + order + preview untouched (take-both pattern).
  await page.getByTestId("merge-take-both-0").click();
  await expect(page.getByTestId("merge-take-both-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nleft\nright");
  await page.getByTestId("merge-take-both-order-0").click();
  await expect(page.getByTestId("merge-take-both-order-0")).toContainText("R to L");
  await expect(preview).toHaveText("base\nright\nleft");

  // Composition smoke: take-all-right composes with the choice, then
  // find composes with the resolution, then apply still receipts.
  await page.getByTestId("merge-take-all-right").click();
  await expect(page.getByTestId("merge-take-right-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nright");
  await expect(page.getByTestId("merge-take-all-status")).toContainText("took right for 1 conflict");
  // Wave-113C: the find panel is default-closed; open it for the legacy
  // find-composes assertions below.
  if ((await page.getByTestId("mergeview/toolbar-find").getAttribute("aria-expanded")) !== "true") {
    await page.getByTestId("mergeview/toolbar-find").click();
  }
  await expect(page.getByTestId("merge/find-input")).toBeVisible();
  await page.getByTestId("merge/find-input").fill("t");
  await page.getByTestId("merge/find-input").press("Enter");
  await expect(page.getByTestId("merge/find-status")).toContainText("match 1 of 2");
  await expect(preview).toHaveText("base\nright");
  mkdirSync(EVID_DIR, { recursive: true });
  await page.getByTestId("merge-out-path").fill(OUT_FILE);
  await page.getByTestId("merge-apply").click();
  const report = page.getByTestId("merge-apply-report");
  await expect(report).toBeVisible({ timeout: 10000 });
  const receipt = ((await report.textContent()) ?? "").trim();
  expect(receipt, "mock receipt names the command").toContain("merge_text_apply");
  expect(receipt, "mock receipt echoes the outPath payload").toContain(OUT_FILE);
  expect(receipt, "mock receipt echoes the resolution payload").toContain("1 resolution");

  // Dark scheme: bracket, hatch, chevrons and no-crash hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await expect(card).toHaveAttribute("data-bracket", "single");
  await expect(panes.nth(1)).toHaveClass(/mv-cell-empty/);
  const darkChevron = await page
    .getByTestId("merge-take-left-0")
    .evaluate((el) => window.getComputedStyle(el).color);
  expect(darkChevron, "chevron stays golden-amber in dark").toBe("rgb(255, 159, 10)");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.emulateMedia({ colorScheme: "light" });

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
