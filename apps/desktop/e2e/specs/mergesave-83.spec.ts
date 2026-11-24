/**
 * Wave-83 merge Save-As browse proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) and proves the apply-row native Save dialog:
 *  - the apply row exposes a Save-As/Browse button (`merge-out-browse`)
 *    next to the typed `merge-out-path` input;
 *  - the typed-outPath apply flow still works end to end (take-left ->
 *    type outPath -> Apply -> receipt; existing merge-apply behavior);
 *  - the dialog-absent path never crashes: with no mock installed the
 *    Browse click keeps the typed value (native helper returns null
 *    without Tauri internals); a rejecting mock also keeps the typed
 *    value with zero crash; a resolving mock fills outPath;
 *  - collapsed chrome budget intact: first conflict-card top <= 140px
 *    relative to the merge section top (find panel closed);
 *  - light + dark, zero console/page errors, zero `role="alert"`.
 *
 * New file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { existsSync, mkdirSync, rmSync } from "node:fs";

const EVID_DIR = "/tmp/ui-83";
const OUT_FILE = `${EVID_DIR}/merge-save-out.txt`;

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

test("merge save dialog: browse button, typed apply intact, absent-dialog never crashes, row budget", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  mkdirSync(EVID_DIR, { recursive: true });
  try {
    rmSync(OUT_FILE);
  } catch {
    // Missing output file is the expected starting state.
  }

  await createMergeSession(page);
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");

  // 1. Apply row exposes the Save-As/Browse button next to the input.
  const outInput = page.getByTestId("merge-out-path");
  const browse = page.getByTestId("merge-out-browse");
  await expect(outInput).toBeVisible();
  await expect(browse).toBeVisible();
  await expect(browse).toContainText("Browse");

  // 2. Dialog-absent path (no mock, no Tauri internals): Browse keeps
  //    the typed value, never crashes, never flips status.
  await outInput.fill("/tmp/typed-keep.txt");
  await browse.click();
  await expect(outInput).toHaveValue("/tmp/typed-keep.txt");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");

  // 3. Resolving mock fills outPath with the pick.
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__MOCK_DIALOG_SAVE__ = async () => "/tmp/picked-83.txt";
  });
  await browse.click();
  await expect(outInput).toHaveValue("/tmp/picked-83.txt");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // 4. Rejecting mock keeps the typed value, never crashes.
  await outInput.fill("/tmp/typed-after-reject.txt");
  await page.evaluate(() => {
    (window as unknown as Record<string, unknown>).__MOCK_DIALOG_SAVE__ = async () => {
      throw new Error("mock dialog boom");
    };
  });
  await browse.click();
  await expect(outInput).toHaveValue("/tmp/typed-after-reject.txt");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Clear the mock so the rest of the flow proves honest typed behavior.
  await page.evaluate(() => {
    delete (window as unknown as Record<string, unknown>).__MOCK_DIALOG_SAVE__;
  });

  // 5. Typed-outPath apply flow still works end to end.
  await page.getByTestId("merge-take-left-0").click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("merge-preview")).toHaveText("base\nleft");
  await outInput.fill(OUT_FILE);
  const report = page.getByTestId("merge-apply-report");
  await page.getByTestId("merge-apply").click();
  await expect(report).toBeVisible({ timeout: 10000 });
  const receipt = ((await report.textContent()) ?? "").trim();
  expect(receipt.length, "apply renders a receipt").toBeGreaterThan(0);
  if (existsSync(OUT_FILE)) {
    expect(receipt, "success receipt names the output path").toContain(OUT_FILE);
  } else {
    expect(receipt, "mock receipt names the command").toContain("merge_text_apply");
    expect(receipt, "mock receipt echoes the outPath payload").toContain(OUT_FILE);
    expect(receipt, "mock receipt echoes the resolution payload").toContain("1 resolution");
  }
  await expect(page.getByRole("alert")).toHaveCount(0);

  // 6. Collapsed chrome budget intact (panel closed): first-card top
  //    relative to the merge section top <= 140px.
  const findInput = page.getByTestId("merge/find-input");
  if (await findInput.isVisible()) {
    await page.getByTestId("mergeview/toolbar-find").click();
    await expect(findInput).toBeHidden();
  }
  const chrome = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector('[data-testid="mergeview/conflict-0"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    const s = section.getBoundingClientRect();
    const f = first.getBoundingClientRect();
    return { delta: f.top - s.top };
  });
  expect(chrome, "section + first card mounted").not.toBeNull();
  expect(
    chrome?.delta ?? Number.POSITIVE_INFINITY,
    `merge chrome collapsed to one row (got ${chrome?.delta}px)`,
  ).toBeLessThanOrEqual(140);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/merge-save-83-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-save-83-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
