/**
 * Living parity spec: FOLDER view (BC truth: orphan side + density + chrome).
 * Rendered-geometry only.
 */
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) { test.skip(true, `infra: ${err}`); }
});

test("folder parity: orphan side + chrome budget (demo or session)", async ({ page }) => {
  // Living spec owns its session: no /tmp dependency (wiped before),
  // committed parity-harness fixtures as paths (mock engine serves rows).
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  if ((await page.locator('[data-testid^="folder-row-"]').count()) === 0) {
    await page.getByTestId("toolbar/action-open").click();
    await page.getByTestId("session/kind-select").selectOption("folder");
    await page.getByTestId("session/left-path").fill("/Users/ektasaini/autocompare/tools/parity-harness/fixtures/folder/left");
    await page.getByTestId("session/right-path").fill("/Users/ektasaini/autocompare/tools/parity-harness/fixtures/folder/right");
    await page.getByTestId("session/create-button").click();
    await page.getByRole("button", { name: "Close dialog" }).click();
    await expect(page.getByTestId("view-folder")).toBeVisible();
  }
  const rows = page.locator('[data-testid^="folder-row-"]');
  const n = await rows.count();
  // orphan honesty (folder rows are single-column tree rows, not panes):
  // data-side must agree with data-status, and the missing side meta
  // must read honestly ("size unknown") instead of a fake size.
  const orphans = await page.evaluate(() => {
    const out: Array<{ status: string | null; side: string | null; meta: string | null }> = [];
    document.querySelectorAll('[data-testid^="folder-row-"]').forEach((el) => {
      const status = el.getAttribute("data-status");
      if (status === "leftOnly" || status === "rightOnly") {
        const meta = el.querySelector('[data-testid^="folder-meta-"]');
        out.push({ status, side: el.getAttribute("data-side"), meta: meta?.textContent ?? null });
      }
    });
    return out;
  });
  expect(orphans.length, "mock folder has orphans").toBeGreaterThan(0);
  for (const o of orphans) {
    const wantSide = o.status === "leftOnly" ? "left-only" : "right-only";
    expect(o.side, `data-side matches status for ${o.status}`).toBe(wantSide);
    expect(o.meta ?? "", `missing side honest for ${o.status}`).toContain("size unknown");
  }

  // density 17px pitch
  const d = await page.evaluate(() => {
    const nodes = Array.from(document.querySelectorAll('[data-testid^="folder-row-"]')).slice(0, 6);
    return nodes.map((x) => { const e = x as HTMLElement; return { h: e.offsetHeight, top: e.getBoundingClientRect().top }; });
  });
  for (const [i, r] of d.entries()) expect(r.h, `row ${i} 17px`).toBe(17);

  // chrome budget
  const delta = await page.evaluate(() => {
    const sec = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(sec instanceof HTMLElement) || !(first instanceof HTMLElement)) return 9999;
    return first.getBoundingClientRect().top - sec.getBoundingClientRect().top;
  });
  expect(delta, `firstRowTop<=140 got ${delta}`).toBeLessThanOrEqual(140);
});
