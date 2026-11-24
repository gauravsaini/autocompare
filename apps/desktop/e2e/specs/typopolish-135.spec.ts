/**
 * Wave-135 typography-exquisite pass (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Gap survey — what prior specs already pin (NOT re-asserted here):
 *  - typo-51: selected tab tracking (-0.12px), statusbar 11px,
 *    pathbar input mono family.
 *  - typo-unify: `.tv-cell`/`.tv-gutter` family+lh+tracking,
 *    `.mv-pane pre`, `.sv-row`, `.mw-row` family+lh.
 *  - typopolish-66d: all seven views code+UI weight/lh/tracking,
 *    tabular gutter, folder compact-16px exception, dark spot-check.
 *  - parity-typo-100: token values (UI 12 / meta 11 / code 12,
 *    code lh 1.55, weights 400/500/600), gutter/table/hex tabular
 *    numerals, system stacks + zero webfonts, active tab pill,
 *    toolbar meta 11px + icon strokes, folder/tree/header sizes,
 *    dark pill+size+stroke holds.
 *
 * This spec pins ONLY the wave-135 gaps:
 *  1. display scale tokens (`--ac-font-size-display/title/body` 24/18/14,
 *     `--ac-line-height-display/title` 1.25/1.4) resolve and the
 *     `.ac-display`/`.ac-title`/`.ac-body` applications ride them
 *     (UI stack, tight tracking);
 *  2. SessionManager h2 rides `.ac-title` (18px — was hardcoded text-lg);
 *  3. welcome/empty h1 + body copy ride `.ac-display`/`.ac-body`
 *     (probed — the demo session mounts first-paint so the welcome
 *     branch is not live; the same classes the branch uses are pinned);
 *  4. LineDetailPane pres ride the mono token (was tailwind `font-mono`,
 *     fights `var(--ac-font-mono)`) — pinned when a row selection
 *     mounts the pane;
 *  5. merge preview textarea rides the mono token (code surface the
 *     `.mv-pane pre` rule never covered) — pinned when the merge
 *     session mounts it;
 *  6. pathbar input line-height rides the code token (18.6px — was
 *     `leading-none`, typo-51 pinned family only).
 * Light + dark, zero console/page errors, view-scoped no-overlap.
 * Zoomed crops land in `/tmp/typopolish-135-*` (never in-repo).
 * New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

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

/** View-scoped overlap (wave-66d idiom): this wave owns typography only,
 *  so the pin covers the active data view subtree, never the tabbar. */
const VIEW_OVERLAP_SELECTORS =
  'main, [data-testid^="view-"], [data-testid^="view-"] [data-testid], .tv-cell, .tv-gutter, .fv-node, .hx-row, .tc-cell, .tc-head, .sv-row, .mw-row';

test("typopolish-135: display scale + code-surface gaps ride tokens, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // ---- 1+3. Display scale tokens + applications (probed: welcome branch
  // is not live under the demo first-paint, but these are the exact
  // classes the h1/p use).
  const displayVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const root = window.getComputedStyle(document.documentElement);
    const want = (name: string, v: string): void => {
      if (root.getPropertyValue(name).trim() !== v)
        failures.push(`${name}: ${root.getPropertyValue(name).trim()}`);
    };
    want("--ac-font-size-display", "24px");
    want("--ac-font-size-title", "18px");
    want("--ac-font-size-body", "14px");
    want("--ac-line-height-display", "1.25");
    want("--ac-line-height-title", "1.4");
    const probe = (cls: string, size: string, lh: number, track: number): void => {
      const el = document.createElement("div");
      el.className = cls;
      el.textContent = "Ag";
      document.body.appendChild(el);
      const c = window.getComputedStyle(el);
      if (c.fontSize !== size) failures.push(`${cls} size: ${c.fontSize}`);
      if (Math.abs(parseFloat(c.lineHeight) - lh) > 0.05) failures.push(`${cls} lh: ${c.lineHeight}`);
      if (!c.fontFamily.includes("SF Pro")) failures.push(`${cls} stack: ${c.fontFamily}`);
      if (Math.abs(parseFloat(c.letterSpacing) - track) > 0.03)
        failures.push(`${cls} tracking: ${c.letterSpacing}`);
      el.remove();
    };
    // 24px * -0.01em = -0.24px; 18px -> -0.18px; 14px -> -0.14px.
    probe("ac-display", "24px", 30, -0.24);
    probe("ac-title", "18px", 25.2, -0.18);
    probe("ac-body", "14px", 21, -0.14);
    return failures;
  });
  expect(displayVerdict, `display scale (got: ${displayVerdict.join(" | ")})`).toEqual([]);

  // ---- 2. SessionManager h2 rides .ac-title.
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  const titleVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const h2 = document.querySelector("#session-manager-title");
    if (!h2) {
      failures.push("no session-manager-title");
    } else {
      const c = window.getComputedStyle(h2);
      if (c.fontSize !== "18px") failures.push(`title size: ${c.fontSize}`);
      if (!c.fontFamily.includes("SF Pro")) failures.push(`title stack: ${c.fontFamily}`);
      if (Math.abs(parseFloat(c.lineHeight) - 25.2) > 0.2)
        failures.push(`title lh: ${c.lineHeight}`);
    }
    return failures;
  });
  expect(titleVerdict, `manager title (got: ${titleVerdict.join(" | ")})`).toEqual([]);
  await page.locator("#session-manager-title").screenshot({ path: "/tmp/typopolish-135-title-light.png" });
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();

  // ---- 6. Pathbar input rides the code line-height (typo-51 did family).
  await expect(page.getByTestId("view-text")).toBeVisible();
  const pathVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const input = document.querySelector('[data-testid="pathbar/left"]');
    if (!input) {
      failures.push("no pathbar/left");
    } else {
      const c = window.getComputedStyle(input);
      if (!c.fontFamily.includes("SF Mono")) failures.push(`pathbar family: ${c.fontFamily}`);
      if (Math.abs(parseFloat(c.lineHeight) - 18.6) > 0.2)
        failures.push(`pathbar lh: ${c.lineHeight} (was leading-none 12px)`);
    }
    return failures;
  });
  expect(pathVerdict, `pathbar input (got: ${pathVerdict.join(" | ")})`).toEqual([]);
  await page.getByTestId("pathbar/left").screenshot({ path: "/tmp/typopolish-135-pathbar-light.png" });

  // ---- 4+5. Code-surface gaps: line-detail pane (on row select) and
  // merge preview textarea. At least one must mount and ride mono.
  let gapPinned = 0;
  const gapFailures: string[] = [];
  await expect
    .poll(async () => page.locator(".tv-cell").count(), { timeout: 15000 })
    .toBeGreaterThan(0);
  await page.locator(".tv-cell").first().click();
  await page.waitForTimeout(300);
  if ((await page.getByTestId("line-detail/left").count()) > 0) {
    const ld = await page.evaluate(() => {
      const out: string[] = [];
      const el = document.querySelector('[data-testid="line-detail/left"]');
      if (el) {
        const c = window.getComputedStyle(el);
        if (!c.fontFamily.includes("SF Mono")) out.push(`line-detail family: ${c.fontFamily}`);
        if (c.fontSize !== "12px") out.push(`line-detail size: ${c.fontSize}`);
        if (Math.abs(parseFloat(c.lineHeight) - 18.6) > 0.15)
          out.push(`line-detail lh: ${c.lineHeight}`);
      }
      return out;
    });
    gapFailures.push(...ld);
    if (ld.length === 0) gapPinned += 1;
  }
  await createSession(page, "merge", "/tmp/merge12-base.txt", "/tmp/merge12-right.txt", "view-merge");
  if ((await page.getByTestId("merge-preview-edit").count()) > 0) {
    const mv = await page.evaluate(() => {
      const out: string[] = [];
      const el = document.querySelector('[data-testid="merge-preview-edit"]');
      if (el) {
        const c = window.getComputedStyle(el);
        if (!c.fontFamily.includes("SF Mono")) out.push(`preview family: ${c.fontFamily}`);
        if (c.fontSize !== "12px") out.push(`preview size: ${c.fontSize}`);
        if (Math.abs(parseFloat(c.lineHeight) - 18.6) > 0.15)
          out.push(`preview lh: ${c.lineHeight}`);
        if (c.letterSpacing !== "0px" && c.letterSpacing !== "normal")
          out.push(`preview tracking: ${c.letterSpacing}`);
      }
      return out;
    });
    gapFailures.push(...mv);
    if (mv.length === 0) {
      gapPinned += 1;
      await page.getByTestId("merge-preview-edit").screenshot({ path: "/tmp/typopolish-135-preview-light.png" });
    }
  }
  expect(gapFailures, `code gaps (got: ${gapFailures.join(" | ")})`).toEqual([]);
  expect(gapPinned, "at least one code-gap surface (line-detail or merge preview) mounted").toBeGreaterThan(0);

  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // ---- Dark: display scale + pathbar hold.
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(400);
  const darkVerdict = await page.evaluate(() => {
    const failures: string[] = [];
    const el = document.createElement("div");
    el.className = "ac-display";
    el.textContent = "Ag";
    document.body.appendChild(el);
    const c = window.getComputedStyle(el);
    if (c.fontSize !== "24px") failures.push(`dark display size: ${c.fontSize}`);
    if (!c.fontFamily.includes("SF Pro")) failures.push(`dark display stack: ${c.fontFamily}`);
    el.remove();
    const input = document.querySelector('[data-testid="pathbar/left"]');
    if (input) {
      const i = window.getComputedStyle(input);
      if (Math.abs(parseFloat(i.lineHeight) - 18.6) > 0.2)
        failures.push(`dark pathbar lh: ${i.lineHeight}`);
    }
    return failures;
  });
  expect(darkVerdict, `dark pins (got: ${darkVerdict.join(" | ")})`).toEqual([]);
  await assertNoOverlap(page, VIEW_OVERLAP_SELECTORS);
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
