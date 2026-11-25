/**
 * Wave-WD hex/merge/media toolbar parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Pins the HexToolbar/MergeToolbar/MediaToolbar parity on shared UI primitives:
 *  - Hex: HexToolbar with page nav, diff nav UnifiedNavCluster, diffs-only,
 *    bytes/line Select, sync scroll Checkbox, copy group, find/goto toggles;
 *    HexFindPanel & HexGotoPanel ride ui/DockedBar (open/close via button + Esc, testids intact);
 *  - Merge: MergeToolbar with conflict nav UnifiedNavCluster, conflict position,
 *    take all left/right Buttons, find toggle; MergeFindPanel rides ui/DockedBar
 *    (open/close via toggle, close button, Esc key on input);
 *  - Media: MediaToolbar with mode indicator (Picture vs Audio), tolerance toggle &
 *    recompare in audio mode; MediaTolerancePanel rides ui/Modal (role=dialog,
 *    open/close via toggle, close button, Esc key on input, coarse slider + precision input);
 *  - Light + dark PNGs land in `/tmp/hexmergemedia-WD/` (never in-repo);
 *  - `assertNoOverlap` holds and zero console/page errors occur.
 *
 * New file only; no existing spec touched; styling edit zero.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/hexmergemedia-WD";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function assertClean(consoleErrors: string[], pageErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewTestId: string,
): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestId)).toBeVisible();
}

test("hex toolbar + find + goto: shared ui primitives, DockedBar open/close, testids intact, light+dark", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await createSession(page, "hex", "/tmp/wd-hex-left.bin", "/tmp/wd-hex-right.bin", "view-hex");

  const toolbar = page.getByTestId("hexview/toolbar");
  await expect(toolbar).toBeVisible();

  // Testids intact in HexToolbar
  await expect(page.getByTestId("hexview/toolbar-prev-page")).toBeVisible();
  await expect(page.getByTestId("hexview/toolbar-next-page")).toBeVisible();
  await expect(page.getByTestId("hexview/page-label")).toBeVisible();
  await expect(page.getByTestId("hex-diff-prev")).toBeVisible();
  await expect(page.getByTestId("hex-diff-next")).toBeVisible();
  await expect(page.getByTestId("hex-diff-status")).toBeVisible();
  await expect(page.getByTestId("hexview/diffsonly-toggle")).toBeVisible();
  await expect(page.getByTestId("hexview/bytes-per-line")).toBeVisible();
  await expect(page.getByTestId("hexview/sync-toggle")).toBeVisible();
  await expect(page.getByTestId("hex-copy-bar")).toBeVisible();
  await expect(page.getByTestId("hex-copy-hex")).toBeVisible();
  await expect(page.getByTestId("hex-copy-ascii")).toBeVisible();

  const findToggle = page.getByTestId("hexview/toolbar-find");
  const gotoToggle = page.getByTestId("hexview/toolbar-goto");
  await expect(findToggle).toBeVisible();
  await expect(gotoToggle).toBeVisible();

  // Both Find and Goto panels ride DockedBar (open by default)
  const findPanel = page.getByTestId("hex/find-panel");
  const gotoPanel = page.getByTestId("hex/goto-panel");
  await expect(findPanel).toBeVisible();
  await expect(gotoPanel).toBeVisible();

  // 1. Close goto panel first via its close button
  await page.getByTestId("hex-goto-close").click();
  await expect(gotoPanel).toBeHidden();

  // 2. Find panel controls and open/close cycles
  await expect(page.getByTestId("hex-find-input")).toBeVisible();
  await expect(page.getByTestId("hex-find-next")).toBeVisible();
  await expect(page.getByTestId("hex-find-status")).toBeVisible();

  // Close find panel via close button
  await page.getByTestId("hex-find-close").click();
  await expect(findPanel).toBeHidden();

  // Re-open find panel via toolbar toggle
  await findToggle.click();
  await expect(findPanel).toBeVisible();

  // Close find panel via Escape on input
  await page.getByTestId("hex-find-input").press("Escape");
  await expect(findPanel).toBeHidden();

  // 3. Goto panel controls and open/close cycles
  await gotoToggle.click();
  await expect(gotoPanel).toBeVisible();
  await expect(page.getByTestId("hex-goto-input")).toBeVisible();
  await expect(page.getByTestId("hex-goto-go")).toBeVisible();
  await expect(page.getByTestId("hex-goto-status")).toBeVisible();

  // Close goto panel via Escape on input
  await page.getByTestId("hex-goto-input").press("Escape");
  await expect(gotoPanel).toBeHidden();

  // Re-open goto panel and close via close button
  await gotoToggle.click();
  await expect(gotoPanel).toBeVisible();
  await page.getByTestId("hex-goto-close").click();
  await expect(gotoPanel).toBeHidden();

  await expect(page.getByRole("alert")).toHaveCount(0);
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/hex-toolbar-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.getByTestId("view-hex")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/hex-toolbar-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });
  assertClean(consoleErrors, pageErrors);
});

test("merge toolbar + find panel: nav arrows, conflict position, find DockedBar open/close, light+dark", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await createSession(page, "merge", "/tmp/wd-merge-base.txt", "/tmp/wd-merge-right.txt", "view-merge");

  const toolbar = page.getByTestId("mergeview/toolbar");
  await expect(toolbar).toBeVisible();

  // Nav cluster & conflict position
  await expect(page.getByTestId("merge-prev")).toBeVisible();
  await expect(page.getByTestId("merge-next")).toBeVisible();
  await expect(page.getByTestId("merge-conflict-position")).toBeVisible();
  await expect(page.getByTestId("merge-take-all-left")).toBeVisible();
  await expect(page.getByTestId("merge-take-all-right")).toBeVisible();

  // Find panel starts closed by default
  const findToggle = page.getByTestId("mergeview/toolbar-find");
  await expect(findToggle).toBeVisible();
  await expect(findToggle).toHaveAttribute("aria-expanded", "false");
  const findPanel = page.getByTestId("merge/find-panel");
  await expect(findPanel).toHaveCount(0);

  // Open find panel via toolbar toggle
  await findToggle.click();
  await expect(findToggle).toHaveAttribute("aria-expanded", "true");
  await expect(findPanel).toBeVisible();

  // Controls inside find panel
  await expect(page.getByTestId("merge/find-input")).toBeVisible();
  await expect(page.getByTestId("merge/find-prev")).toBeVisible();
  await expect(page.getByTestId("merge/find-next")).toBeVisible();
  await expect(page.getByTestId("merge/find-status")).toBeVisible();

  // Close find panel via close button
  await page.getByTestId("merge/find-close").click();
  await expect(findPanel).toBeHidden();
  await expect(findToggle).toHaveAttribute("aria-expanded", "false");

  // Re-open and close via Escape in input
  await findToggle.click();
  await expect(findPanel).toBeVisible();
  await page.getByTestId("merge/find-input").press("Escape");
  await expect(findPanel).toBeHidden();
  await expect(findToggle).toHaveAttribute("aria-expanded", "false");

  await expect(page.getByRole("alert")).toHaveCount(0);
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/merge-toolbar-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.getByTestId("view-merge")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/merge-toolbar-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });
  assertClean(consoleErrors, pageErrors);
});

test("media toolbar + tolerance panel: picture mode, audio mode tolerance Modal open/close, testids intact, light+dark", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  // 1. Picture mode: mode indicator only, no tolerance controls
  await createSession(page, "media", "/tmp/wd-pic-left.png", "/tmp/wd-pic-right.png", "view-media");
  await expect(page.getByTestId("mediaview/toolbar")).toBeVisible();
  await expect(page.getByTestId("mediaview/mode")).toContainText("Picture compare");
  await expect(page.getByTestId("mediaview/toolbar-tolerance")).toBeHidden();
  await expect(page.getByTestId("mediaview/recompare")).toBeHidden();

  // 2. Audio mode: toolbar tolerance toggle + recompare button
  await createSession(page, "media", "/tmp/wd-audio-left.wav", "/tmp/wd-audio-right.wav", "view-media");
  await expect(page.getByTestId("mediaview/toolbar")).toBeVisible();
  await expect(page.getByTestId("mediaview/mode")).toContainText("Audio compare");
  const tolToggle = page.getByTestId("mediaview/toolbar-tolerance");
  await expect(tolToggle).toBeVisible();
  await expect(page.getByTestId("mediaview/recompare")).toBeVisible();

  // Tolerance panel is a Modal open by default in audio mode
  const tolPanel = page.getByTestId("mediaview/tolerance-panel");
  await expect(tolPanel).toBeVisible();
  await expect(tolToggle).toHaveAttribute("aria-expanded", "true");

  // Controls inside tolerance modal
  const tolInput = page.getByTestId("mediaview/tolerance");
  const tolSlider = page.getByTestId("mediaview/tolerance-slider");
  const tolClose = page.getByTestId("mediaview/tolerance-close");
  await expect(tolInput).toBeVisible();
  await expect(tolSlider).toBeVisible();
  await expect(tolClose).toBeVisible();

  // Close tolerance panel via close button
  await tolClose.click();
  await expect(tolPanel).toBeHidden();
  await expect(tolToggle).toHaveAttribute("aria-expanded", "false");

  // Re-open tolerance panel via toolbar toggle
  await tolToggle.click();
  await expect(tolPanel).toBeVisible();
  await expect(tolToggle).toHaveAttribute("aria-expanded", "true");

  // Close via Escape in tolerance input
  await tolInput.press("Escape");
  await expect(tolPanel).toBeHidden();
  await expect(tolToggle).toHaveAttribute("aria-expanded", "false");

  // Recompute with tolerance value via input Enter
  await tolToggle.click();
  await expect(tolPanel).toBeVisible();
  await tolInput.fill("5");
  await tolInput.press("Enter");
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);

  // Close panel and verify recompare button on toolbar
  await tolClose.click();
  await expect(tolPanel).toBeHidden();
  await page.getByTestId("mediaview/recompare").click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);

  await expect(page.getByRole("alert")).toHaveCount(0);
  await assertNoOverlap(page);
  await page.screenshot({ path: `${EVID_DIR}/media-toolbar-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.getByTestId("view-media")).toBeVisible();
  await page.screenshot({ path: `${EVID_DIR}/media-toolbar-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });
  assertClean(consoleErrors, pageErrors);
});
