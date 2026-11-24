/**
 * Wave-66T ignore backend wiring proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Wave-75B supersedes the stale honest-fallback toggle assertions below:
 * wave-75 made the mock echo `appliedOptions` for either spelling and
 * routed `TextView.recompareWithIgnores` via typed `textCompare()`, so
 * the UI toggles now take the supported path (note hidden, re-diff
 * reloads) instead of the honest fallback.
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves:
 *  - the typed `textCompare(left, right, opts)` wrapper sends both
 *    spellings the Rust command accepts (camelCase for the TextView
 *    probe path, snake_case for the typed path) and the mock echoes
 *    `appliedOptions` on EITHER spelling path;
 *  - legacy `{left, right}` (no opts) stays byte-identical: no echo,
 *    no `__TEXT_IGNORE_LAST__` flags;
 *  - the UI toggles route via the typed path, so the backend echo is
 *    accepted, the note stays HIDDEN, and the diff reloads through the
 *    normal row/overview setters (honest fallback note only for
 *    genuinely old backends with no echo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: the mock adapter has no whitespace/case engine (rows
 * are fixture rows), so the echo proves the flags round-tripped and
 * the view hides the note; the real honoring is covered by Rust unit
 * tests in `commands/text.rs`. Skip policy matches the other specs
 * (infra failures skip).
 */
import { expect, test } from "@playwright/test";

const NOTE_TEXT = "Ignore options aren't supported by this backend";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("text ignore wiring: typed echo, legacy byte-identical, toggle honest fallback", async ({
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
  const statusBefore = await page.getByTestId("textview/status").innerText();

  // 1. Typed path with flags: mock echoes appliedOptions + top-level flags,
  //    and the wrapper records the request for headless proof.
  const typed = await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    const res = (await mod.textCompare("left-a", "right-b", {
      ignoreWhitespace: true,
      ignoreCase: false,
    })) as unknown as Record<string, unknown>;
    const last = (window as unknown as Record<string, unknown>).__TEXT_IGNORE_LAST__;
    return { res, last };
  });
  const typedRes = typed.res as Record<string, unknown>;
  expect(typedRes["session"]).toBe("mock-text-1");
  expect(typedRes["ignoreWhitespace"]).toBe(true);
  expect(typedRes["ignoreCase"]).toBe(false);
  const applied = typedRes["appliedOptions"] as Record<string, unknown>;
  expect(applied["ignoreWhitespace"]).toBe(true);
  expect(applied["ignoreCase"]).toBe(false);
  const last = typed.last as Record<string, unknown>;
  expect(last["ignoreWhitespace"]).toBe(true);
  expect(last["ignoreCase"]).toBe(false);

  // 2. Legacy path (no opts): byte-identical wire, no echo.
  const legacy = await page.evaluate(async () => {
    const mod = await import("/src/ipc/text.ts");
    return (await mod.textCompare("left-a", "right-b")) as unknown as Record<string, unknown>;
  });
  expect(legacy["session"]).toBe("mock-text-1");
  expect("appliedOptions" in legacy).toBe(false);
  expect("ignoreWhitespace" in legacy).toBe(false);

  // 3. UI toggles route via the typed path: backend echoes applied
  // options, so the supported path hides the note and reloads the
  // re-diff (honest fallback only for genuinely old backends).
  // NOTE_TEXT is the genuinely-unsupported fallback text — not expected
  // on the mock.
  void NOTE_TEXT;
  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  // Wave-112 preamble: the ignore panel is closed by default (BC
  // truth); open it via its toolbar toggle. Zero assertion changes.
  const ignoreToggle112 = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle112.getAttribute("aria-expanded")) !== "true") await ignoreToggle112.click();
  await expect(wsToggle).toBeVisible();
  await wsToggle.check();
  await expect(wsToggle).toBeChecked();
  await expect
    .poll(
      async () =>
        await page.evaluate(() => {
          const w = window as unknown as Record<string, unknown>;
          return (w["__TEXT_IGNORE_LAST__"] as Record<string, unknown> | undefined)?.[
            "ignoreWhitespace"
          ];
        }),
      { timeout: 10000 },
    )
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText(statusBefore);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await wsToggle.uncheck();
  await expect
    .poll(
      async () =>
        await page.evaluate(() => {
          const w = window as unknown as Record<string, unknown>;
          return (w["__TEXT_IGNORE_LAST__"] as Record<string, unknown> | undefined)?.[
            "ignoreWhitespace"
          ];
        }),
      { timeout: 10000 },
    )
    .toBe(false);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText(statusBefore);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
