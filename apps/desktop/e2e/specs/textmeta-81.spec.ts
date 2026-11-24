/**
 * Wave-81 path-bar meta line proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the Beyond Compare style per-side meta line (`timestamp ·
 * bytes · lines` under the path inputs, testids `textview/meta`,
 * `textview/meta-left`, `textview/meta-right`):
 *  - the mock omits `leftMeta`/`rightMeta` (no file stats in mock), so
 *    the meta line HIDES honestly: `textview/meta` count is 0, the demo
 *    still boots (`textview/status` shows 200 / 644 rows), zero console
 *    errors / page errors (no crash, no fabricated values);
 *  - the pure exported `formatTextMeta` formatter is proven in-page
 *    (null meta / empty meta -> null = hide; full meta -> locale stamp
 *    + `N bytes` + `M lines`; partial meta renders known parts only;
 *    invalid numbers stay hidden);
 *  - first-row budget: `textview/row-0` still mounts at the top of its
 *    list container (`top <= 140px`, the hidden line adds no stack);
 *  - light + dark PNGs land in `/tmp/ui-81/` (never in-repo).
 *
 * Wave-82 follow-up: the backend starts echoing `leftMeta`/`rightMeta`
 * on the `text_compare` summary, which activates the render path (this
 * spec then upgrades to pin the visible line: left/right text, ` · `
 * separators, dark-mode paint). Until then this spec pins the honest
 * fallback green. Skip policy matches the other specs (infra failures
 * skip).
 */
import { expect, test } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-81";

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

test("meta line hides honestly without backend meta, formatter covers render path", async ({
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

  // 1. Honest fallback: the mock carries no file meta, so the line hides.
  await expect(page.getByTestId("textview/meta")).toHaveCount(0);
  await expect(page.getByTestId("textview/meta-left")).toHaveCount(0);
  await expect(page.getByTestId("textview/meta-right")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // 2. Pure formatter proof (the render path wave-82 activates).
  const formatted = await page.evaluate(async () => {
    const mod = await import("/src/components/views/TextView.tsx");
    const fmt = mod.formatTextMeta as (
      meta: { bytes?: number; mtimeUnix?: number } | null | undefined,
      lines: number | null,
    ) => string | null;
    return {
      nullMeta: fmt(null, 10),
      undefinedMeta: fmt(undefined, 10),
      emptyMeta: fmt({}, 10),
      emptyMetaNoLines: fmt({}, null),
      full: fmt({ bytes: 5195, mtimeUnix: 1700000000 }, 123),
      bytesOnly: fmt({ bytes: 4932 }, 97),
      timeOnly: fmt({ mtimeUnix: 1700000000 }, 97),
      noLines: fmt({ bytes: 100 }, null),
      invalidBytes: fmt({ bytes: -5 }, 10),
      nanTime: fmt({ mtimeUnix: Number.NaN }, 10),
    };
  });
  expect(formatted.nullMeta, "null meta hides").toBeNull();
  expect(formatted.undefinedMeta, "undefined meta hides").toBeNull();
  expect(formatted.emptyMeta, "fieldless meta hides (lines alone stay in the status line)").toBeNull();
  expect(formatted.emptyMetaNoLines, "fieldless meta without lines hides").toBeNull();
  expect(formatted.invalidBytes, "negative bytes hide").toBeNull();
  expect(formatted.nanTime, "NaN mtime hides").toBeNull();
  expect(formatted.full, "full meta renders").not.toBeNull();
  expect(formatted.full ?? "", "full meta names bytes").toContain("5195 bytes");
  expect(formatted.full ?? "", "full meta names lines").toContain("123 lines");
  expect(formatted.full ?? "", "full meta joins with middots").toContain(" · ");
  expect(formatted.bytesOnly, "bytes-only meta renders known parts").toBe("4932 bytes · 97 lines");
  expect(formatted.timeOnly, "time-only meta renders stamp + lines").toContain("97 lines");
  expect(formatted.noLines, "meta without lines still renders bytes").toBe("100 bytes");

  // 3. First-row budget: the hidden line adds no vertical stack.
  const budget = await page.evaluate(() => {
    const row = document.querySelector('[data-testid="textview/row-0"]');
    if (!(row instanceof HTMLElement)) return null;
    const host = row.parentElement;
    if (!(host instanceof HTMLElement)) return null;
    const r = row.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    return { top: r.top - h.top, absTop: r.top };
  });
  expect(budget, "row-0 mounted").not.toBeNull();
  expect(
    budget?.top ?? Number.POSITIVE_INFINITY,
    `first row at list top (got ${budget?.top}px, abs ${budget?.absTop}px)`,
  ).toBeLessThanOrEqual(140);

  await assertNoOverlap(page);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/textmeta-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/textmeta-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
