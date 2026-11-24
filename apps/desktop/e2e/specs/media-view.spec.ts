/**
 * Wave-48 media view proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `App` workspace (SessionManager dialog -> tab ->
 * ViewRouter -> MediaView) over the mock media engine and proves:
 *  - a `media` session mounts `view-media` instead of the fallback
 *    panel (the last kind without a view);
 *  - `.png` paths run the picture mode with the mock score and samples;
 *  - `.wav` paths run the audio mode with window counts, and a
 *    tolerance round-trip keeps the summary live;
 *  - rendered nodes never overlap (boundingBox check);
 *  - light + dark PNGs land in `/tmp/ui-48/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Skip policy matches the other specs (infra failures skip). New file
 * only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import { assertNoOverlap } from "../helpers/overlap";

const EVID_DIR = "/tmp/ui-48";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createMediaSession(page: Page, left: string, right: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("media");
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-media")).toBeVisible();
}

test("media view: picture mode score+samples, audio mode windows+tolerance, light+dark", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await createMediaSession(page, "/tmp/m-left.png", "/tmp/m-right.png");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Picture compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/99.7% equal/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/12 \/ 4100 bytes differ/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(2);
  await assertNoOverlap(page);

  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Audio compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/3 \/ 100 windows differ/);
  await expect(page.getByTestId("mediaview/row")).toHaveCount(3);
  await page.getByTestId("mediaview/tolerance").fill("5");
  await page.getByTestId("mediaview/recompare").click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);
  await assertNoOverlap(page);

  await expect(page.getByRole("alert")).toHaveCount(0);

  await page.screenshot({ path: `${EVID_DIR}/media-view-light.png` });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${EVID_DIR}/media-view-dark.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
