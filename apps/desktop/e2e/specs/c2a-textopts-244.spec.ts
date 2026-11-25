/**
 * Wave-C2a text options end-to-end (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows, 200 loaded) and
 * proves the engine-ready text options actually re-diff end to end:
 *  - the Ignore menu carries an "Ignore blank lines" checkbox
 *    (`textview/ignore-blank-lines`, unchecked by default); checking it
 *    re-issues `text_compare` through the typed `textCompare` wrapper
 *    with `ignoreBlankLines: true` (visible via the
 *    `__TEXT_IGNORE_LAST__` probe the wrapper maintains), the row
 *    status stays mounted, and unchecking round-trips to `false`;
 *  - the Format menu carries a "Patience diff" toggle
 *    (`textview/algorithm-patience`, unchecked = Myers by default);
 *    checking it re-issues `text_compare` with `algorithm: "patience"`,
 *    and unchecking drops back to the legacy exact wire;
 *  - the typed `textCompare(left, right, opts)` wrapper forwards both
 *    spellings the Rust command accepts (`algorithm`, `ignoreBlankLines`
 *    + snake_case twins) and records them on the probe;
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: the mock adapter has no Patience/blank engine (rows are
 * fixture rows), so the probe + mounted status prove the flags
 * round-trip and reload; the real collapsing is pinned by the Rust unit
 * test `c2a_algorithm_and_blank_lines_round_trip` in
 * `src-tauri/src/commands/text.rs`. No geometry asserts, no screenshots.
 * Skip policy matches the other specs (infra failures skip).
 */
import { expect, test } from "@playwright/test";

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

async function lastOpts(page: {
  evaluate: <T>(fn: () => T) => Promise<T>;
}): Promise<Record<string, unknown> | null> {
  return page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const last = w["__TEXT_IGNORE_LAST__"];
    if (typeof last !== "object" || last === null) return null;
    return last as Record<string, unknown>;
  });
}

test("c2a text options: patience re-compares, blank-lines round-trips", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Demo text diff boots automatically (mock IPC serves real engine rows).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");

  // 1. Blank-lines checkbox: unchecked default, check re-compares, uncheck restores.
  const ignoreToggle = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle.getAttribute("aria-expanded")) !== "true") await ignoreToggle.click();
  const blankToggle = page.getByTestId("textview/ignore-blank-lines");
  await expect(blankToggle).toBeVisible();
  await expect(blankToggle).not.toBeChecked();
  await blankToggle.check();
  await expect(blankToggle).toBeChecked();
  await expect
    .poll(async () => (await lastOpts(page))?.["ignoreBlankLines"], { timeout: 10000 })
    .toBe(true);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await blankToggle.uncheck();
  await expect(blankToggle).not.toBeChecked();
  await expect
    .poll(async () => (await lastOpts(page))?.["ignoreBlankLines"], { timeout: 10000 })
    .toBe(false);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  // Close the Ignore panel via Esc: the Modal scrim blocks toolbar
  // clicks while open (the fallback clicks the toolbar button directly).
  await page.keyboard.press("Escape");
  await expect(blankToggle).toBeHidden();

  // 2. Patience toggle: unchecked default (Myers), check re-compares, uncheck restores.
  const formatToggle = page.getByTestId("textview/toolbar-format");
  if ((await formatToggle.getAttribute("aria-expanded")) !== "true") await formatToggle.click();
  const patienceToggle = page.getByTestId("textview/algorithm-patience");
  await expect(patienceToggle).toBeVisible();
  await expect(patienceToggle).not.toBeChecked();
  await patienceToggle.check();
  await expect(patienceToggle).toBeChecked();
  await expect
    .poll(async () => (await lastOpts(page))?.["algorithm"], { timeout: 10000 })
    .toBe("patience");
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  await patienceToggle.uncheck();
  await expect(patienceToggle).not.toBeChecked();
  await expect
    .poll(async () => (await lastOpts(page))?.["algorithm"], { timeout: 10000 })
    .toBe(undefined);
  await expect(page.getByTestId("textview/status")).toContainText("200 / 644 rows");
  // Close the Format panel via Esc before the typed-wrapper probes.
  await page.keyboard.press("Escape");
  await expect(patienceToggle).toBeHidden();

  // 3. Typed wrapper forwards both spellings onto the probe.
  const typedPatience = (await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    return (await mod.textCompare("left-a", "right-b", {
      algorithm: "patience",
    })) as unknown as Record<string, unknown>;
  })) as Record<string, unknown>;
  expect(typedPatience["session"]).toBeDefined();
  const typedBlank = (await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    return (await mod.textCompare("left-a", "right-b", {
      ignore_blank_lines: true,
    })) as unknown as Record<string, unknown>;
  })) as Record<string, unknown>;
  expect(typedBlank["session"]).toBeDefined();
  await expect
    .poll(async () => (await lastOpts(page))?.["ignoreBlankLines"], { timeout: 10000 })
    .toBe(true);

  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
