/**
 * Wave-45 merge take-all proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict
 * merge pair and proves Beyond Compare style bulk resolution:
 *  - `merge-take-all-left` resolves every conflict to the left (the
 *    per-conflict take-left goes `aria-pressed`, the preview renders
 *    `base\nleft` with no markers) and reports an honest count
 *    (`merge-take-all-status`: `took left for 1 conflict`);
 *  - `merge-take-all-right` flips every conflict to the right with its
 *    own honest receipt;
 *  - a later per-conflict click clears the bulk receipt so it never
 *    describes a stale choice;
 *  - keyboard: Enter on the focused take-all button resolves too;
 *  - light + dark PNGs land in `/tmp/ui-45/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-45";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createMergeSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/merge12-base.txt");
  await page.getByTestId("session/right-path").fill("/tmp/merge12-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-merge")).toBeVisible();
}

test("merge take-all: bulk left/right resolve, count receipt, per-conflict clears, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMergeSession(page);
  const preview = page.getByTestId("merge-preview");
  const allLeft = page.getByTestId("merge-take-all-left");
  const allRight = page.getByTestId("merge-take-all-right");
  const bulkStatus = page.getByTestId("merge-take-all-status");
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");
  await expect(allLeft).toBeVisible();
  await expect(allRight).toBeVisible();
  await expect(bulkStatus).toHaveCount(0);

  // Take all left: every conflict resolves left, preview + receipt honest.
  await allLeft.click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nleft");
  await expect(preview).not.toContainText("<<<<<<<");
  await expect(bulkStatus).toContainText("took left for 1 conflict");

  // Take all right: flips the resolution with its own receipt.
  await allRight.click();
  await expect(page.getByTestId("merge-take-right-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).not.toContainText("<<<<<<<");
  await expect(bulkStatus).toContainText("took right for 1 conflict");

  // A per-conflict click supersedes the bulk receipt (never stale).
  await page.getByTestId("merge-take-left-0").click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nleft");
  await expect(bulkStatus).toHaveCount(0);

  // Keyboard: Enter on the focused take-all button resolves too.
  await allRight.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("merge-take-right-0")).toHaveAttribute("aria-pressed", "true");
  await expect(bulkStatus).toContainText("took right for 1 conflict");

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/merge-takeall-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-takeall-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
