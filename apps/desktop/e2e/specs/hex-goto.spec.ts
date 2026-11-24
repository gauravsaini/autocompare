/**
 * Wave-42 hex goto-offset proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) over the embedded 11-byte `h_*` pair ("hello
 * world" vs "hallo world", single page) and proves Beyond Compare style
 * offset jumping:
 *  - a decimal offset ("5") jumps the pager and selects `hexview/byte-5`
 *    (detail readout follows, status reports the jump);
 *  - `0x`-hex ("0x8") resolves to the same byte as decimal "8";
 *  - Enter in the input jumps too;
 *  - garbage ("zz", empty) and out-of-range ("11", "999", "-1") render
 *    honest inline notes -- never a crash, zero `role=alert`;
 *  - light + dark PNGs land in `/tmp/ui-42/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-42";

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
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("hex");
  await page.getByTestId("session/left-path").fill("/tmp/hex-19-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-19-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

test("hex goto: decimal + 0x-hex jump and select, invalid + range notes, light+dark", async ({
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
  await expect(view).toContainText("files differ");
  await expect(view).toContainText("11 bytes");

  const input = page.getByTestId("hex-goto-input");
  const go = page.getByTestId("hex-goto-go");
  const status = page.getByTestId("hex-goto-status");
  await expect(input).toBeVisible();
  await expect(go).toBeVisible();
  await expect(status).toContainText("0 to 10");

  // Decimal jump selects the byte: pager stays page-0, detail follows.
  await input.fill("5");
  await go.click();
  await expect(status).toContainText("jumped to offset 5");
  await expect(page.getByTestId("hexview/page-0")).toBeVisible();
  await expect(page.getByTestId("hexview/byte-5")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hex-detail-offset")).toContainText("0x00000005");

  // 0x-hex resolves to the same byte as decimal; Enter jumps too.
  await input.fill("0x8");
  await input.press("Enter");
  await expect(status).toContainText("jumped to offset 8");
  await expect(page.getByTestId("hexview/byte-8")).toHaveAttribute("data-selected", "true");
  await expect(page.getByTestId("hexview/byte-5")).not.toHaveAttribute("data-selected", "true");
  await input.fill("8");
  await go.click();
  await expect(page.getByTestId("hexview/byte-8")).toHaveAttribute("data-selected", "true");

  // Garbage + out-of-range render honest inline notes, selection keeps.
  await input.fill("zz");
  await go.click();
  await expect(status).toContainText("decimal or 0x-hex");
  await input.fill("");
  await go.click();
  await expect(status).toContainText("decimal or 0x-hex");
  await input.fill("11");
  await go.click();
  await expect(status).toContainText("out of range (0-10)");
  await input.fill("999");
  await go.click();
  await expect(status).toContainText("out of range (0-10)");
  await input.fill("-1");
  await go.click();
  await expect(status).toContainText("decimal or 0x-hex");
  await expect(page.getByTestId("hexview/byte-8")).toHaveAttribute("data-selected", "true");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hex-goto-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hex-goto-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
