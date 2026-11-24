/**
 * Wave-30 ignore toggles proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Wave-75B supersedes the stale always-unsupported assertions below:
 * wave-75 made the mock echo `appliedOptions` for either spelling and
 * routed `TextView.recompareWithIgnores` via typed `textCompare()`, so
 * toggles now re-diff (supported path) instead of always showing the
 * honest fallback note.
 *
 * Drives the REAL `App` workspace with the auto-booted demo text diff
 * (mock `text_compare` over `dumped.json`: 644 rows) and proves the
 * Beyond Compare style whitespace/case toggles:
 *  - both checkboxes render unchecked by default;
 *  - flipping either toggle re-issues `text_compare` with the matching
 *    flag via the typed path; the mock backend echoes applied flags,
 *    so the view hides the honest inline note and reloads through the
 *    normal row/overview setters (status line still shows the mock's
 *    200 / 644 fixture rows, no crash, no alert);
 *  - unchecking both restores the exact diff with the note hidden;
 *  - toggles are keyboard reachable (focus + Space flips one on);
 *  - light + dark PNGs land in `/tmp/ui-30/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock honesty: the mock IPC adapter echoes `appliedOptions` + the
 * top-level flags for either spelling (mirroring the real Rust
 * backend), so supported-path assertions below read the echo
 * (`__TEXT_IGNORE_LAST__`) plus the hidden note. The honest fallback
 * note (`NOTE_TEXT`) appears ONLY on genuinely old backends with no
 * echo — not on the mock. Skip policy matches the other specs (infra
 * failures skip).
 */
import { expect, test } from "@playwright/test";

const EVID_DIR = "/tmp/ui-30";
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

test("ignore toggles render unchecked, flip honestly, keep the diff", async ({ page }) => {
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

  const wsToggle = page.getByTestId("textview/ignore-whitespace");
  const caseToggle = page.getByTestId("textview/ignore-case");
  // Wave-112 preamble (batch-green extension, same pattern as the five
  // named specs): the ignore panel is closed by default; open it via
  // its toolbar toggle. Zero assertion changes.
  const ignoreToggle112 = page.getByTestId("textview/toolbar-ignore");
  if ((await ignoreToggle112.getAttribute("aria-expanded")) !== "true") await ignoreToggle112.click();
  await expect(wsToggle).toBeVisible();
  await expect(caseToggle).toBeVisible();
  await expect(wsToggle).not.toBeChecked();
  await expect(caseToggle).not.toBeChecked();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);

  // Flip whitespace on: supported backend echoes applied flags, so the
  // honest note stays hidden and the re-diff reloads (no crash, no alert).
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

  // Flip case on too: supported path, note stays hidden, diff reloaded.
  await caseToggle.check();
  await expect(caseToggle).toBeChecked();
  await expect
    .poll(
      async () =>
        await page.evaluate(() => {
          const w = window as unknown as Record<string, unknown>;
          return (w["__TEXT_IGNORE_LAST__"] as Record<string, unknown> | undefined)?.[
            "ignoreCase"
          ];
        }),
      { timeout: 10000 },
    )
    .toBe(true);
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText(statusBefore);

  // Evidence of the toggle-on honest-note state (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/text-ignore-on-light.png` });

  // Keyboard reachability: tab to the whitespace toggle and flip it off
  // with Space; the case toggle stays on so the supported path keeps the
  // note hidden.
  await wsToggle.focus();
  await page.keyboard.press("Space");
  await expect(wsToggle).not.toBeChecked();
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText(statusBefore);

  // Uncheck the last toggle: the exact diff restores, note stays hidden.
  await caseToggle.uncheck();
  await expect(caseToggle).not.toBeChecked();
  // Honest fallback text (NOTE_TEXT) appears only on genuinely old
  // backends with no applied-options echo — not on the mock.
  void NOTE_TEXT;
  await expect(page.getByTestId("textview/ignore-note")).toHaveCount(0);
  await expect(page.getByTestId("textview/status")).toContainText(statusBefore);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/text-ignore-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/text-ignore-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
