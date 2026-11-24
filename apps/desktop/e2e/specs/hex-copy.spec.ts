/**
 * Wave-36 hex export-selection proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) with a hex session and proves:
 *  - Copy-hex + Copy-ASCII buttons copy the selected byte (left preferred):
 *    offset 1 is left `65` / `e`, and the inline receipt (`hex-copy-status`)
 *    confirms with either `copied ...` or an honest `clipboard ...` fallback
 *    carrying the value, depending on headless clipboard capability;
 *  - with no selection the buttons copy the current page summary
 *    (`files differ ... 11 bytes ... page 1/1`);
 *  - Ctrl+C on a focused byte copies hex (same receipt contract);
 *  - the wave-23 footer detail (`hex-detail-*`) and the wave-19 find UI
 *    (`hex-find-input` / `hex-find-next` / `hex-find-status`) keep working;
 *  - light + dark PNGs land in `/tmp/ui-36/` (never in-repo).
 *
 * The mock IPC adapter has no hex fixtures (it resolves `[]`), so HexView
 * pages over small embedded byte-identical copies of the
 * `tools/parity-harness/fixtures/h_left.bin` ("hello world") and
 * `h_right.bin` ("hallo world") pair; every assertion below reads the DOM.
 * Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-36";

// Headless webkit is slow to boot under load; match the hex-find budget.
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

async function createHexSession(page: Page): Promise<void> {
  // Wave-10 boots the demo text session; clear it for deterministic counts.
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/hex-36-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-36-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

/** Receipt confirms inline: `copied <kind>: <value>` or honest fallback. */
async function expectCopyReceipt(
  status: ReturnType<Page["getByTestId"]>,
  kind: string,
  value: string,
): Promise<void> {
  await expect(status).toContainText(kind);
  await expect(status).toContainText(value);
  const text = (await status.textContent()) ?? "";
  expect(
    text.includes("copied") || text.includes("clipboard"),
    `receipt must confirm copy or note fallback (got: ${text})`,
  ).toBe(true);
}

test("hex copy: selected byte hex+ascii, summary fallback, Ctrl+C, footer+find intact, light+dark", async ({
  page,
  browserName,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createHexSession(page);
  const view = page.getByTestId("view-hex");

  // Embedded h_* pair: left "hello world" vs right "hallo world" (differ at
  // offset 1), 11 bytes on a single page.
  await expect(view).toContainText("files differ");
  await expect(view).toContainText("11 bytes");
  await expect(view).toContainText("page 1/1");

  const copyHex = page.getByTestId("hex-copy-hex");
  const copyAscii = page.getByTestId("hex-copy-ascii");
  const copyStatus = page.getByTestId("hex-copy-status");
  await expect(copyHex).toBeVisible();
  await expect(copyAscii).toBeVisible();
  await expect(copyStatus).toBeVisible();

  // No selection yet: buttons copy the current page summary.
  await copyHex.click();
  await expectCopyReceipt(copyStatus, "summary", "11 bytes");

  // Select the diff byte at offset 1: left 0x65 'e', right 0x61 'a'.
  const byte1 = page.getByTestId("hexview/byte-1");
  await expect(byte1).toBeVisible();
  await byte1.click();
  await expect(byte1).toHaveAttribute("data-selected", "true");

  // Copy hex of the selection (left preferred: `65`).
  await copyHex.click();
  await expectCopyReceipt(copyStatus, "hex", "65");

  // Copy ASCII of the selection (left preferred: `e`).
  await copyAscii.click();
  await expectCopyReceipt(copyStatus, "ascii", "e");

  // Keyboard: Ctrl/Cmd-C on the focused byte copies hex.
  await byte1.focus();
  if (browserName === "webkit") {
    await page.keyboard.press("Meta+C");
  } else {
    await page.keyboard.press("Control+C");
  }
  await expectCopyReceipt(copyStatus, "hex", "65");

  // Wave-23 footer detail still reports the selection.
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(1)");
  await expect(page.getByTestId("hex-detail-left")).toContainText("65");
  await expect(page.getByTestId("hex-detail-right")).toContainText("61");
  await expect(page.getByTestId("hex-detail-flag")).toContainText("diff");

  // Wave-19 find still works: `6c` ("l") first matches at offset 2.
  await page.getByTestId("hex-find-input").fill("6c");
  await page.getByTestId("hex-find-next").click();
  await expect(page.getByTestId("hex-find-status")).toContainText("match at offset 2");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hex-copy-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hex-copy-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
