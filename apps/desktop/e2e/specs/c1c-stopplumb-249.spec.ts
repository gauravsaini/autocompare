/**
 * C1c stop-plumb: Toolbar Stop ab real backend abort karta hai (MOCK IPC
 * ONLY, `VITE_MOCK_IPC=1`, fresh port, `--workers=1`, webkit).
 *
 * C1b me Stop sirf tab `folderSession` prop pata hone par `folder_cancel`
 * bulata tha, par koi host ye prop pass nahi karta tha — isliye Stop
 * hamesha honest-note par girta tha (C1b ka documented OFF). Ye wave App
 * (`runComparePaths` receipt) se real session id upar ChromeShell tak
 * plumb karta hai aur Stop par `folder_cancel(session)` fire karta hai:
 *  1. Folder session, busy + known session -> Stop `folder_cancel`
 *     bhejta hai, note me `cancel requested (session mock-folder-1)`.
 *     (Do compare: pehla session seed karta hai, dusra busy window me
 *     Stop pakadta hai.)
 *  2. Text session (koi folder session nahi) -> Stop honest no-op, note
 *     me `cannot be cancelled`, busy apne aap settle hota hai.
 *  3. Idle Stop disabled rehta hai, title byte-identical, koi note nahi.
 *
 * Static pins (C1b precedent: `c1-p0-243.spec.ts` source pins) + runtime
 * proof. No geometry asserts. Infra failures skip (matches other specs).
 * NEW file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createFolderSession(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill("/tmp/c1c-left");
  await page.getByTestId("session/right-path").fill("/tmp/c1c-right");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden({ timeout: 15000 });
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 15000 });
}

async function runsText(page: Page): Promise<string> {
  return ((await page.getByTestId("toolbar/compare-runs").textContent()) ?? "").trim();
}

test("c1c-stopplumb-249: Stop while busy with known session fires folder_cancel", async ({
  page,
}) => {
  // Static pin: App plumbs the real folder session id down to Stop.
  const here = path.dirname(fileURLToPath(import.meta.url));
  const appSrc = fs.readFileSync(path.resolve(here, "../../src/App.tsx"), "utf8");
  expect(appSrc, "App captures the folder_compare session receipt").toContain("setFolderSessionId(fc.session)");
  expect(appSrc, "App forwards the session to the chrome shell").toContain("folderSession={sessionKind");
  const shellSrc = fs.readFileSync(
    path.resolve(here, "../../src/components/chrome/ChromeShell.tsx"),
    "utf8",
  );
  expect(shellSrc, "ChromeShell forwards the session to Toolbar").toContain(
    "folderSession={folderSession",
  );
  const toolbarSrc = fs.readFileSync(
    path.resolve(here, "../../src/components/chrome/Toolbar.tsx"),
    "utf8",
  );
  expect(toolbarSrc, "Stop path calls folder_cancel with the session").toContain(
    "folderCancel(target)",
  );

  await createFolderSession(page);
  const compare = page.getByTestId("toolbar/action-compare");
  const stop = page.getByTestId("toolbar/action-compare-stop");
  await expect(compare).toBeEnabled({ timeout: 15000 });

  // Pehla compare: session seed hota hai (mock-folder-1).
  await compare.click();
  await expect(page.getByTestId("toolbar/compare-busy")).toBeVisible({ timeout: 5000 });
  await expect(page.getByTestId("toolbar/compare-busy")).toBeHidden({ timeout: 15000 });
  await expect.poll(() => runsText(page), { timeout: 10000 }).toBe("runs: 1");

  // Dusra compare: busy window me Stop dabao -> real folder_cancel.
  await compare.click();
  await expect(page.getByTestId("toolbar/compare-busy")).toBeVisible({ timeout: 5000 });
  await expect(stop).toBeEnabled({ timeout: 5000 });
  await expect(stop).toHaveAttribute("title", "Stop");
  await stop.click();
  const note = page.getByTestId("toolbar/compare-note");
  await expect(note).toBeVisible({ timeout: 5000 });
  await expect(note).toContainText("folder compare cancel requested (session mock-folder-1)");

  // Running compare apne aap settle hota hai: busy clear, run counted.
  await expect(page.getByTestId("toolbar/compare-busy")).toBeHidden({ timeout: 15000 });
  await expect.poll(() => runsText(page), { timeout: 10000 }).toBe("runs: 2");
  await expect(stop).toBeDisabled();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("c1c-stopplumb-249: Stop without known session keeps honest note", async ({
  page,
}) => {
  // Demo text session auto-boot hoti hai: koi folder session nahi, isliye
  // Stop honest no-op rehta hai (C1b fallback intact).
  const compare = page.getByTestId("toolbar/action-compare");
  const stop = page.getByTestId("toolbar/action-compare-stop");
  await expect(compare).toBeEnabled({ timeout: 15000 });

  await compare.click();
  await expect(page.getByTestId("toolbar/compare-busy")).toBeVisible({ timeout: 5000 });
  await expect(stop).toBeEnabled({ timeout: 5000 });
  await stop.click();
  const note = page.getByTestId("toolbar/compare-note");
  await expect(note).toBeVisible({ timeout: 5000 });
  await expect(note).toContainText("cannot be cancelled");
  await expect(note).not.toContainText("cancel requested");

  await expect(page.getByTestId("toolbar/compare-busy")).toBeHidden({ timeout: 15000 });
  await expect.poll(() => runsText(page), { timeout: 10000 }).toBe("runs: 1");
  await expect(stop).toBeDisabled();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("c1c-stopplumb-249: idle Stop stays disabled with honest title", async ({
  page,
}) => {
  const stop = page.getByTestId("toolbar/action-compare-stop");
  await expect(stop).toBeDisabled({ timeout: 15000 });
  await expect(stop).toHaveAttribute(
    "title",
    "Stop (no compare running: backend has no abort IPC)",
  );
  await expect(page.getByTestId("toolbar/compare-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
});
