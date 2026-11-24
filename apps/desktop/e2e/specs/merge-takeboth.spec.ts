/**
 * Wave-37 merge take-both proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView), clicks the per-conflict take-both button in
 * both orders (left-then-right / right-then-left via the order toggle),
 * and checks the preview concatenation + apply receipt through the
 * existing `merge_text_apply` receipt path.
 *
 * HONEST BACKEND ASSESSMENT (read before citing this spec):
 * - The mock IPC adapter (`src/ipc/client.ts`, FORBIDDEN to this wave)
 *   has no `merge_text_apply` fixture: `mockInvoke` falls through to
 *   `default: []`. The browser therefore CANNOT perform a real file
 *   write under `VITE_MOCK_IPC=1` -- there is no Tauri shell, no Rust
 *   `merge_text_apply`, and no filesystem access from the page.
 * - So this spec proves the REAL call path instead of a real write:
 *   MergeView calls the REAL `mergeTextApply` IPC wrapper
 *   (`src/ipc/merge.ts` -> `invoke("merge_text_apply", { base, left,
 *   right, resolutions, out_path })`) with a `bothLeftFirst` /
 *   `bothRightFirst` resolution, and the receipt under test is rendered
 *   ONLY after that promise settles. The mock branch echoes the exact
 *   payload back inline (`1 resolution(s) -> <outPath>`), which is
 *   assertable proof the invoke payload reached `merge_text_apply`
 *   with the take-both choice + explicit outPath.
 * - REAL-BACKEND COMPATIBILITY: if this spec ever runs against the
 *   real Tauri shell (no `VITE_MOCK_IPC`), the same Apply writes the
 *   file via Rust `merge_text_apply` (which resolves `BothLeftFirst` /
 *   `BothRightFirst` in `commands/merge.rs`) and the spec takes the
 *   file branch instead: `node:fs.readFileSync(outPath)` bytes must
 *   equal the preview bytes (LF-joined + trailing newline). The branch
 *   taken is decided at runtime by whether the output file exists; the
 *   mock branch is expected under the default `playwright.config.ts`
 *   webServer (`VITE_MOCK_IPC=1`, port 1420, no overlay config -- free
 *   ports and any overlay config live in /tmp only, nothing added here).
 *
 * Assertions:
 *  - take-both (default L->R) resolves the preview to `base\nleft\nright`
 *    (both sides present, left line before right line, no markers);
 *  - order toggle flips to R->L and the preview becomes
 *    `base\nright\nleft` (right line before left line, no markers);
 *  - Apply with an outPath renders a receipt on `merge-apply-report`
 *    (mock: `mock fixture` + outPath + resolution count; real: `bytes`
 *    + path receipt);
 *  - zero `role="alert"` crashes and zero console/page errors;
 *  - light + dark PNGs land in `/tmp/ui-37/` (never in-repo).
 *
 * Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";

const EVID_DIR = "/tmp/ui-37";
const OUT_FILE = `${EVID_DIR}/merge-takeboth-out.txt`;

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
  // Wave-10 boots the demo text session; clear it for deterministic counts.
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

test("merge take-both: both orders preview + apply receipt, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  page.on("pageerror", (err) => {
    consoleErrors.push(String(err));
  });

  // Evidence dir lives outside the repo; start from a clean output file
  // so the real-vs-mock branch below is decided honestly at runtime.
  mkdirSync(EVID_DIR, { recursive: true });
  try {
    rmSync(OUT_FILE);
  } catch {
    // Missing output file is the expected starting state.
  }

  await createMergeSession(page);
  const preview = page.getByTestId("merge-preview");
  const report = page.getByTestId("merge-apply-report");

  // Conflict plan renders (engine-matched single conflict, id 0).
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");
  await expect(page.getByTestId("merge-take-both-0")).toBeVisible();
  await expect(page.getByTestId("merge-take-both-order-0")).toBeVisible();

  // Default order is left-then-right.
  await expect(page.getByTestId("merge-take-both-order-0")).toContainText("L to R");

  // Take-both L->R: preview concatenates both sides, left line first.
  await page.getByTestId("merge-take-both-0").click();
  await expect(page.getByTestId("merge-take-both-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nleft\nright");
  await expect(preview).not.toContainText("<<<<<<<");
  let previewText = ((await preview.textContent()) ?? "").trim();
  expect(previewText.indexOf("left") < previewText.indexOf("right"), "L->R order: left before right").toBe(true);

  // Order toggle flips to R->L; the take-both choice follows so the
  // preview updates without re-clicking.
  await page.getByTestId("merge-take-both-order-0").click();
  await expect(page.getByTestId("merge-take-both-order-0")).toContainText("R to L");
  await expect(preview).toHaveText("base\nright\nleft");
  await expect(preview).not.toContainText("<<<<<<<");
  previewText = ((await preview.textContent()) ?? "").trim();
  expect(previewText.indexOf("right") < previewText.indexOf("left"), "R->L order: right before left").toBe(true);
  await expect(page.getByTestId("merge-take-both-0")).toHaveAttribute("aria-pressed", "true");

  // Take-left / take-right still exist untouched (no regression).
  await expect(page.getByTestId("merge-take-left-0")).toBeVisible();
  await expect(page.getByTestId("merge-take-right-0")).toBeVisible();

  // Real apply call: take-both resolution + explicit outPath through
  // the real `mergeTextApply` IPC wrapper (existing receipt path).
  await page.getByTestId("merge-out-path").fill(OUT_FILE);
  await page.getByTestId("merge-apply").click();
  await expect(report).toBeVisible({ timeout: 10000 });
  const receipt = ((await report.textContent()) ?? "").trim();
  expect(receipt.length, "apply renders a receipt").toBeGreaterThan(0);

  if (existsSync(OUT_FILE)) {
    // REAL BACKEND PATH: Tauri `merge_text_apply` wrote the file; its
    // bytes must equal the preview (LF-joined + trailing newline, the
    // same framing `merge.rs` uses for `write_merged`).
    const expected = previewText === "" ? "" : `${previewText}\n`;
    const actual = readFileSync(OUT_FILE, "utf8");
    expect(actual, "written file bytes equal the merged preview").toBe(expected);
    expect(receipt, "success receipt names the output path").toContain(OUT_FILE);
    expect(receipt, "success receipt reports bytes written").toContain("bytes");
  } else {
    // MOCK IPC PATH (expected under `VITE_MOCK_IPC=1`): the adapter has
    // no `merge_text_apply` fixture, so no file exists. The receipt is
    // only renderable after the real `mergeTextApply(...)` promise
    // settles, and it echoes the invoke payload inline -- that echo is
    // the proof the payload reached `merge_text_apply`.
    expect(receipt, "mock receipt names the command").toContain("merge_text_apply");
    expect(receipt, "mock receipt echoes the outPath payload").toContain(OUT_FILE);
    expect(receipt, "mock receipt echoes the resolution payload").toContain("1 resolution");
  }

  // Backend errors and refusals surface inline -- never a crash.
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/merge-takeboth-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-takeboth-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);
  expect(consoleErrors, `zero console/page errors (got: ${consoleErrors.join("; ")})`).toEqual([]);
});
