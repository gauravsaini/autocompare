/**
 * Wave-19 hex find + jump proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> HexView) with a hex session and proves:
 *  - the view renders the find control (`hex-find-input`,
 *    `hex-find-next`, `hex-find-status`) next to the pager;
 *  - typing hex (`6c`, raw with optional spaces) + Find next jumps the
 *    pager to the matching page (`hexview/page-0`) and highlights exactly
 *    the matched bytes (`.hx-find-match` count == needle length), with the
 *    status line reporting the match offset;
 *  - repeated Find next continues forward (2 -> 3 -> 9) and wraps past the
 *    end back to the first match (`(wrapped)` in the status);
 *  - Enter in the input searches too, tolerating spaces + uppercase hex;
 *  - an absent needle reports `no match`; bad input (`zz`, odd digits,
 *    empty) reports an inline error -- never a crash, zero `role=alert`;
 *  - light + dark PNGs land in `/tmp/ui-19/` (never in-repo).
 *
 * The mock IPC adapter has no hex fixtures (it resolves `[]`), so HexView
 * pages + finds over small embedded byte-identical copies of the
 * `tools/parity-harness/fixtures/h_left.bin` ("hello world") and
 * `h_right.bin` ("hallo world") pair, planned locally by the view; every
 * assertion below reads the DOM, never the network. The embedded pair is
 * 11 bytes (a single page), so the pager jump is asserted as page-0 +
 * offset + highlight. Skip policy matches the other specs (infra failures
 * skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-19";

// Headless webkit is slow to boot under load; match the hunk-nav budget.
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
  await page.getByTestId("session/left-path").fill("/tmp/hex-19-left.bin");
  await page.getByTestId("session/right-path").fill("/tmp/hex-19-right.bin");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-hex")).toBeVisible();
}

test("hex find: jump to match offset, next continues + wraps, invalid input inline, light+dark", async ({
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
  await expect(view).toContainText("page 1/1");

  const input = page.getByTestId("hex-find-input");
  const next = page.getByTestId("hex-find-next");
  const status = page.getByTestId("hex-find-status");
  await expect(input).toBeVisible();
  await expect(next).toBeVisible();
  await expect(status).toBeVisible();

  // Fresh query starts at the page end (256) and wraps to the first `l`.
  await input.fill("6c");
  await next.click();
  await expect(status).toContainText("match at offset 2");
  await expect(page.getByTestId("hexview/page-0")).toBeVisible();
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  // Repeated Find next continues forward through the same needle.
  await next.click();
  await expect(status).toContainText("match at offset 3");
  await expect(status).not.toContainText("wrapped");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  await next.click();
  await expect(status).toContainText("match at offset 9");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  // Past the last match wraps back to the first one.
  await next.click();
  await expect(status).toContainText("match at offset 2");
  await expect(status).toContainText("wrapped");
  await expect(view.locator(".hx-find-match")).toHaveCount(1);

  // Enter searches too; spaces + uppercase hex are tolerated ("lo" @ 3).
  await input.fill(" 6C 6F ");
  await input.press("Enter");
  await expect(status).toContainText("match at offset 3");
  await expect(view.locator(".hx-find-match")).toHaveCount(2);

  // Absent needle reports no match without crashing.
  await input.fill("ff");
  await next.click();
  await expect(status).toContainText("no match");
  await expect(view.locator(".hx-find-match")).toHaveCount(0);

  // Invalid input reports inline errors without touching IPC: non-hex,
  // odd digits, then empty -- the view stays up throughout.
  await input.fill("zz");
  await next.click();
  await expect(status).toContainText("Invalid hex");
  await expect(view).toBeVisible();

  await input.fill("abc");
  await next.click();
  await expect(status).toContainText("even number");
  await expect(view).toBeVisible();

  await input.fill("");
  await next.click();
  await expect(status).toContainText("Enter hex bytes");
  await expect(view).toBeVisible();
  await expect(input).toBeVisible();

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/hex-find-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/hex-find-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
