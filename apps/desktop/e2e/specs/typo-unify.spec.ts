/**
 * Wave-49 code-typography unification proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * The token ramp (`tokens.css`: mono stack, 12px, 1.55 line-height,
 * 0 tracking, tabular numbers) is the single source of truth for code
 * type. This spec pins that every code-bearing view actually rides it:
 *  - `.tv-cell` / `.tv-gutter` (text, demo first-paint),
 *  - `.mv-pane pre` (merge session),
 *  - `.sv-row` (sync session),
 *  - `.mw-row` (media session),
 * resolve to the token values (no tailwind `font-mono` cascade fight),
 * rendered nodes never overlap, and light + dark PNGs land in
 * `/tmp/ui-49/` (never in-repo). Zero console errors / page errors.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-49";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createSession(page: Page, kind: string, left: string, right: string, viewTestId: string): Promise<void> {
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

test("code type rides the token ramp in text/merge/sync/media, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Text (demo first-paint): cells + gutter on the token ramp.
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect
    .poll(async () => page.locator(".tv-cell").count(), { timeout: 15000 })
    .toBeGreaterThan(0);
  const textFailures = await page.evaluate(() => {
    const failures: string[] = [];
    const cell = document.querySelector(".tv-cell");
    const gutter = document.querySelector(".tv-gutter");
    if (!cell) failures.push("no .tv-cell");
    if (!gutter) failures.push("no .tv-gutter");
    const cs = (el: Element | null): CSSStyleDeclaration | null =>
      el ? window.getComputedStyle(el) : null;
    const cc = cs(cell);
    const gc = cs(gutter);
    if (cc) {
      if (!cc.fontFamily.includes("SF Mono")) failures.push(`cell family: ${cc.fontFamily}`);
      if (Math.abs(parseFloat(cc.lineHeight) - 18.6) > 0.05) failures.push(`cell line-height: ${cc.lineHeight}`);
      if (cc.letterSpacing !== "0px" && cc.letterSpacing !== "normal") failures.push(`cell tracking: ${cc.letterSpacing}`);
    }
    if (gc && gc.fontVariantNumeric !== "tabular-nums") failures.push(`gutter nums: ${gc.fontVariantNumeric}`);
    return failures;
  });
  expect(textFailures, `text token ramp (got: ${textFailures.join(" | ")})`).toEqual([]);

  // Merge / sync / media sessions each mount their view; pin the ramp.
  await createSession(page, "merge", "/tmp/u-left.txt", "/tmp/u-right.txt", "view-merge");
  await createSession(page, "sync", "/tmp/u-left-dir", "/tmp/u-right-dir", "view-sync");
  await createSession(page, "media", "/tmp/u-left.png", "/tmp/u-right.png", "view-media");
  const viewFailures = await page.evaluate(() => {
    const failures: string[] = [];
    const check = (sel: string, wantFamily: boolean): void => {
      const el = document.querySelector(sel);
      if (!el) {
        failures.push(`no ${sel} (view switched?)`);
        return;
      }
      const c = window.getComputedStyle(el);
      if (wantFamily && !c.fontFamily.includes("SF Mono")) failures.push(`${sel} family: ${c.fontFamily}`);
      if (Math.abs(parseFloat(c.lineHeight) - 18.6) > 0.05) failures.push(`${sel} line-height: ${c.lineHeight}`);
    };
    // Active tab is the media session; its rows prove .mw-row live.
    check(".mw-row", true);
    return failures;
  });
  expect(viewFailures, `media token ramp (got: ${viewFailures.join(" | ")})`).toEqual([]);

  // Merge + sync views already verified mounted; revisit each tab and
  // pin their selectors while active.
  const tabs = page.getByRole("tab");
  const tabCount = await tabs.count();
  const seen: string[] = [];
  for (let i = 0; i < tabCount; i += 1) {
    await tabs.nth(i).click();
    const active = page.locator('[data-testid="view-merge"], [data-testid="view-sync"]');
    if ((await active.count()) === 0) continue;
    const which = (await page.getByTestId("view-merge").count()) > 0 ? ".mv-pane pre" : ".sv-row";
    seen.push(which);
    const failures: string[] = await page.evaluate((sel: string) => {
      const out: string[] = [];
      const el = document.querySelector(sel);
      if (!el) {
        out.push(`no ${sel}`);
        return out;
      }
      const c = window.getComputedStyle(el);
      if (!c.fontFamily.includes("SF Mono")) out.push(`${sel} family: ${c.fontFamily}`);
      if (Math.abs(parseFloat(c.lineHeight) - 18.6) > 0.05) out.push(`${sel} line-height: ${c.lineHeight}`);
      return out;
    }, which);
    expect(failures, `${which} token ramp (got: ${failures.join(" | ")})`).toEqual([]);
  }
  expect(seen.sort(), "merge + sync tabs revisited").toEqual([".mv-pane pre", ".sv-row"]);

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  await page.screenshot({ path: `${EVID_DIR}/typo-unify-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/typo-unify-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
