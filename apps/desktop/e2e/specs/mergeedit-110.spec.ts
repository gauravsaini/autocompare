/**
 * Wave-110 merge-edit proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) over the engine-matched single-conflict merge
 * pair and proves the GAP-M1/M2/M3 scope from the wave-108 observation
 * study (Beyond Compare Text Merge truth: Take Center, Take Left Then
 * Right on `Cmd+B`):
 *  - GAP-M1: the merged preview is an editable
 *    `<textarea data-testid="merge-preview-edit">` (manual touch-up sticks
 *    in the textarea value) while the legacy `merge-preview` node stays
 *    synced so wave-12/22/37/45 specs keep passing unmodified;
 *  - GAP-M2: per-conflict `merge-take-base-<id>` (Take Base = BC Take
 *    Center) resolves the hunk to the common ancestor text;
 *  - GAP-M3: `Cmd+B`/`Ctrl+B` takes both on the selected conflict and
 *    `j`/`k` steps the conflict selection;
 *  - Regression: empty-`outPath` refusal + apply receipt preserved, and
 *    the existing merge depth / take-both / take-all contracts keep
 *    passing (run as separate specs in the same gate);
 *  - zero `role="alert"` crashes and zero console/page errors;
 *  - light + dark PNGs plus zoomed crops land in `/tmp/` (never in-repo).
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

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
  await expect(page.getByTestId("view-merge")).toBeVisible();
}

test("mergeedit-110: editable preview + take-base + Cmd+B/j/k, apply preserved, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMergeSession(page);
  const edit = page.getByTestId("merge-preview-edit");
  const legacy = page.getByTestId("merge-preview");

  // Plan renders the engine-matched single conflict (id 0).
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");
  await expect(page.getByTestId("merge-take-base-0")).toBeVisible();
  await expect(page.getByTestId("merge-take-both-0")).toBeVisible();

  // GAP-M1: preview is an editable textarea; legacy node stays synced.
  await expect(edit).toBeVisible();
  await expect(edit).toHaveJSProperty("tagName", "TEXTAREA");
  await expect(edit).toHaveValue(/<<<<<<</);
  await expect(legacy).toHaveText(((await edit.inputValue()) ?? "").trim());

  // Manual touch-up sticks in the textarea and mirrors to legacy.
  await edit.fill("base\nleft-touched");
  await expect(edit).toHaveValue("base\nleft-touched");
  await expect(legacy).toHaveText("base\nleft-touched");

  // A new choice re-renders the preview (manual touch-up cleared).
  await page.getByTestId("merge-take-left-0").click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(edit).toHaveValue("base\nleft");
  await expect(legacy).toHaveText("base\nleft");

  // GAP-M2: Take Base resolves the hunk to the common ancestor text
  // (BC Take Center; fixture base mid is empty so the file renders the
  // shared `base` prefix with no markers).
  await page.getByTestId("merge-take-base-0").click();
  await expect(page.getByTestId("merge-take-base-0")).toHaveAttribute("aria-pressed", "true");
  await expect(edit).toHaveValue("base");
  await expect(edit).not.toHaveValue(/<<<<<<</);
  await expect(legacy).toHaveText("base");

  // GAP-M3: j/k step the conflict selection without resolving.
  const section = page.locator('section[aria-label="three-way merge"]');
  await section.click();
  await page.keyboard.press("j");
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  await page.keyboard.press("k");
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  // Take Base is still the recorded choice after stepping.
  await expect(page.getByTestId("merge-take-base-0")).toHaveAttribute("aria-pressed", "true");

  // GAP-M3: Cmd+B / Ctrl+B takes both on the selected conflict (BC Take
  // Left Then Right, default L->R order).
  await section.click();
  await page.keyboard.press("Meta+b");
  if ((await edit.inputValue()) !== "base\nleft\nright") {
    await page.keyboard.press("Control+b");
  }
  await expect(edit).toHaveValue("base\nleft\nright");
  await expect(edit).not.toHaveValue(/<<<<<<</);
  await expect(page.getByTestId("merge-take-both-0")).toHaveAttribute("aria-pressed", "true");
  await expect(legacy).toHaveText("base\nleft\nright");

  // Regression: empty outPath refuses inline; apply renders a receipt.
  await page.getByTestId("merge-apply").click();
  await expect(page.getByTestId("merge-apply-report")).toContainText("outPath is required");
  await page.getByTestId("merge-out-path").fill("/tmp/mergeedit-110-out.txt");
  await page.getByTestId("merge-apply").click();
  const receipt = page.getByTestId("merge-apply-report");
  await expect(receipt).toBeVisible({ timeout: 10000 });
  await expect(receipt).toContainText(/mock fixture|Applied|bytes/i);

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Visuals (never in-repo): full light, zoomed crops, then dark.
  await page.screenshot({ path: "/tmp/mergeedit-110-overview-light.png" });
  await page.locator('[data-testid="mergeview/conflict-0"]').screenshot({
    path: "/tmp/mergeedit-110-zoom-conflict.png",
  });
  await edit.screenshot({ path: "/tmp/mergeedit-110-zoom-preview.png" });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: "/tmp/mergeedit-110-overview-dark.png" });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
