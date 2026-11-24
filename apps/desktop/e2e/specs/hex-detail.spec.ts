/**
 * Wave-23 hex selected-byte detail proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) with a hex session and proves:
 *  - clicking a hex byte selects it (`hexview/byte-<offset>` gains
 *    `data-selected`) and the footer detail bar (`hex-detail`) shows the
 *    offset (0x + decimal), the left byte (hex + printable ascii),
 *    the right byte, and a same/diff badge;
 *  - clicking a diff byte (offset 1: left `65 'e'` vs right `61 'a'`)
 *    reports the `diff` badge, while offset 0 (`68 'h'` both sides)
 *    reports `same`;
 *  - keyboard ArrowRight/ArrowLeft moves the selection by one byte
 *    (1 -> 2 -> 1) with the footer following;
 *  - a find-jump to the same page keeps the selection (offset still
 *    visible); the mock fixture is a single 11-byte page so the pager
 *    Prev/Next stays disabled -- there is no off-page to clear to, and
 *    the spec asserts that honestly instead of forcing a page change;
 *  - light + dark PNGs land in `/tmp/ui-23/` (never in-repo).
 *
 * The mock IPC adapter has no hex fixtures (it resolves `[]`), so HexView
 * pages over small embedded byte-identical copies of the
 * `tools/parity-harness/fixtures/h_left.bin` ("hello world") and
 * `h_right.bin` ("hallo world") pair; every assertion below reads the DOM.
 * Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-23";

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
  await page.getByTestId("session/left-path").fill("/tmp/hex-23-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-23-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

test("hex detail: click selects byte, footer offset+both sides+badge, keyboard moves, find keeps, light+dark", async ({
  page,
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

  const detail = page.getByTestId("hex-detail");
  await expect(detail).toBeVisible();
  await expect(detail).toContainText("Click a byte");

  // Click the diff byte at offset 1: left 0x65 'e', right 0x61 'a'.
  const byte1 = page.getByTestId("hexview/byte-1");
  await expect(byte1).toBeVisible();
  await byte1.click();
  await expect(byte1).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("0x00000001");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(1)");
  await expect(page.getByTestId("hex-detail-left")).toContainText("65");
  await expect(page.getByTestId("hex-detail-left")).toContainText("e");
  await expect(page.getByTestId("hex-detail-right")).toContainText("61");
  await expect(page.getByTestId("hex-detail-right")).toContainText("a");
  await expect(page.getByTestId("hex-detail-flag")).toContainText("diff");

  // A same byte (offset 0, 'h' both sides) reports the `same` badge and
  // moves the selection off offset 1.
  const byte0 = page.getByTestId("hexview/byte-0");
  await byte0.click();
  await expect(byte0).toHaveAttribute("data-selected", "true");
  await expect(byte1).not.toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(0)");
  await expect(page.getByTestId("hex-detail-left")).toContainText("68");
  await expect(page.getByTestId("hex-detail-right")).toContainText("68");
  await expect(page.getByTestId("hex-detail-flag")).toContainText("same");

  // Keyboard: ArrowRight/ArrowLeft moves the selection one byte at a time.
  await byte1.click();
  await expect(byte1).toHaveAttribute("data-selected", "true");
  await byte1.focus();
  await page.keyboard.press("ArrowRight");
  const byte2 = page.getByTestId("hexview/byte-2");
  await expect(byte2).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(2)");
  await page.keyboard.press("ArrowLeft");
  await expect(byte1).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(1)");
  await expect(page.getByTestId("hex-detail-flag")).toContainText("diff");

  // Find-jump to the same (only) page keeps the still-visible selection:
  // search `6c` ("l", first match at offset 2) while byte 1 is selected.
  await page.getByTestId("hex-find-input").fill("6c");
  await page.getByTestId("hex-find-next").click();
  await expect(page.getByTestId("hex-find-status")).toContainText("match at offset 2");
  await expect(page.getByTestId("hexview/page-0")).toBeVisible();
  await expect(byte1).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("(1)");

  // Honest single-page check: the 11-byte mock fixture fits on page 1/1,
  // so Prev/Next are disabled and there is no off-page selection to clear.
  await expect(view).toContainText("page 1/1");
  await expect(view.getByRole("button", { name: "Prev", exact: true })).toBeDisabled();
  await expect(view.getByRole("button", { name: "Next", exact: true })).toBeDisabled();
  await expect(byte1).toHaveAttribute("data-selected", "true");

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hex-detail-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hex-detail-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
