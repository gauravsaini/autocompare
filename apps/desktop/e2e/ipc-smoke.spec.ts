/**
 * Wave 1R IPC smoke (REAL SHELL ONLY).
 *
 * Runs inside the Tauri WebView (e.g. `tauri dev` + WebDriver), NOT under
 * plain `npm run dev`: it calls the real `invoke` pipeline with no
 * `VITE_MOCK_IPC` adapter, so every assertion below exercises the Rust
 * `#[tauri::command]` handlers registered in `src-tauri/src/lib.rs`.
 *
 * File-backed commands need host paths; pass them via env:
 *   SMOKE_LEFT=<abs path> SMOKE_RIGHT=<abs path>
 *   e.g. tools/parity-harness/fixtures/t_left.txt + t_right.txt
 *
 * Skips (does not fail) when Tauri internals are absent, so the default
 * `test:e2e` web run stays green.
 */
import { expect, test } from "@playwright/test";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  const hasTauri = await page.evaluate(() => "__TAURI_INTERNALS__" in window);
  test.skip(!hasTauri, "real-shell IPC smoke needs the Tauri WebView");
});

test("ipc smoke: registry answers session/settings with no args", async ({ page }) => {
  const sessions = await page.evaluate(async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<unknown[]>("session_list");
  });
  expect(Array.isArray(sessions)).toBe(true);

  const settings = await page.evaluate(async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<Record<string, unknown>>("settings_get");
  });
  expect(typeof settings).toBe("object");
});

test("ipc smoke: text_compare over real fixture files", async ({ page }) => {
  const left = process.env.SMOKE_LEFT;
  const right = process.env.SMOKE_RIGHT;
  test.skip(!left || !right, "set SMOKE_LEFT/SMOKE_RIGHT to fixture files");
  const summary = await page.evaluate(
    async ({ left, right }: { left: string; right: string }) => {
      const { invoke } = await import("@tauri-apps/api/core");
      return invoke<{
        session: string;
        leftLines: number;
        rightLines: number;
        rows: number;
        exact: boolean;
      }>("text_compare", { left, right });
    },
    { left, right },
  );
  expect(typeof summary.session).toBe("string");
  expect(summary.session.length).toBeGreaterThan(0);
  expect(summary.rows).toBeGreaterThan(0);
  expect(summary.exact).toBe(true);

  const rows = await page.evaluate(async (session: string) => {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<{ rows: unknown[]; total: number }>("text_get_rows", {
      session,
      start: 0,
      limit: 10,
    });
  }, summary.session);
  expect(rows.total).toBe(summary.rows);
  expect(rows.rows.length).toBeLessThanOrEqual(10);
});
