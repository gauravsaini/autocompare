/**
 * Wave-12 merge-depth proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MergeView) with a merge session and proves depth:
 *  - the view loads the bundled `m_*` 3-way fixtures (base `"base\n"`,
 *    left `"base\nleft\n"`, right `"base\nright\n"`) and renders 3 panes
 *    (left / base / right), a conflict count, and per-conflict
 *    take-left / take-right controls;
 *  - the UI conflict count matches the engine plan: the headless engine
 *    (`MergeSession::new` over the same `m_*` bytes) yields exactly
 *    1 conflict (id 0, base `[]`, left `["left"]`, right `["right"]`,
 *    1 clean block), and the view renders exactly that (count label ==
 *    card count == 1, pane sides `"left"` / `"right"`);
 *  - take-left / take-right change the merged preview output
 *    (markers -> left lines -> right lines);
 *  - prev/next conflict navigation (buttons + `j`/`k` keyboard) works;
 *  - light + dark PNGs land in `/tmp/ui-12/` (never in-repo).
 *
 * The mock IPC adapter has no `merge_text_plan` fixture (it resolves
 * `[]`), so MergeView plans the embedded byte-identical `m_*` copies and
 * labels the source inline; every assertion below reads the DOM, never
 * the network. Skip policy matches the other specs (infra failures skip).
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-12";

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

test("merge depth: engine-matched conflict count, 3 panes, take-left changes output, nav, light+dark", async ({
  page,
}) => {
  await createMergeSession(page);
  const view = page.getByTestId("view-merge");

  // 3 panes (left / base / right) render the engine conflict sides.
  await expect(page.getByTestId("merge-pane-left")).toBeVisible();
  await expect(page.getByTestId("merge-pane-base")).toBeVisible();
  await expect(page.getByTestId("merge-pane-right")).toBeVisible();
  await expect(page.getByTestId("merge-pane-left")).toContainText("left");
  await expect(page.getByTestId("merge-pane-right")).toContainText("right");

  // Conflict count matches the engine plan: label == card count == 1
  // (engine `MergeSession` over the same bytes: 1 conflict, id 0).
  await expect(page.getByTestId("merge-conflict-count")).toContainText("1 conflict");
  const cards = page.locator('[data-testid^="mergeview/conflict-"]');
  await expect.poll(async () => cards.count(), { timeout: 10000 }).toBe(1);
  await expect(view.locator("#merge-conflict-0")).toBeVisible();
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");

  // Per-conflict take-left / take-right controls exist for conflict 0.
  await expect(page.getByTestId("merge-take-left-0")).toBeVisible();
  await expect(page.getByTestId("merge-take-right-0")).toBeVisible();

  // Unresolved preview shows standard conflict markers over engine sides.
  const preview = page.getByTestId("merge-preview");
  await expect(preview).toContainText("<<<<<<<");
  await expect(preview).toContainText("=======");

  // Take-left changes the output: markers resolve to the left lines.
  await page.getByTestId("merge-take-left-0").click();
  await expect(page.getByTestId("merge-take-left-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nleft");
  await expect(preview).not.toContainText("<<<<<<<");

  // Take-right changes the output again: left lines resolve to right lines.
  await page.getByTestId("merge-take-right-0").click();
  await expect(page.getByTestId("merge-take-right-0")).toHaveAttribute("aria-pressed", "true");
  await expect(preview).toHaveText("base\nright");
  await expect(preview).not.toContainText("<<<<<<<");

  // Conflict navigation: buttons + keyboard wrap on the single conflict.
  await expect(page.getByTestId("merge-prev")).toBeVisible();
  await expect(page.getByTestId("merge-next")).toBeVisible();
  await page.getByTestId("merge-next").click();
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  await page.getByTestId("merge-prev").click();
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  await page.locator('section[aria-label="three-way merge"]').click();
  await page.keyboard.press("j");
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");
  await page.keyboard.press("k");
  await expect(page.getByTestId("merge-conflict-position")).toContainText("Conflict 1 of 1");

  // Apply without an output path refuses inline (backend rule surfaced,
  // never a crash); with a path the mock fixture degrades inline too.
  await page.getByTestId("merge-apply").click();
  await expect(page.getByTestId("merge-apply-report")).toContainText("outPath is required");
  await page.getByTestId("merge-out-path").fill("/tmp/merge-12-out.txt");
  await page.getByTestId("merge-apply").click();
  await expect(page.getByTestId("merge-apply-report")).toContainText("mock fixture");

  await expect(page.getByRole("alert")).toHaveCount(0);

  // Light + dark evidence (never in-repo).
  await page.screenshot({ path: `${EVID_DIR}/merge-depth-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/merge-depth-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);
});
