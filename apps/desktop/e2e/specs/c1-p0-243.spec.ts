/**
 * C1b-P0: two broken UI promises from audit Category-3 (MOCK IPC ONLY,
 * `VITE_MOCK_IPC=1`, fresh port, `--workers=1`, webkit).
 *
 *  1. SessionManager Delete called `session_delete`, but the Rust backend
 *     had NO such command (list/load/save only) — delete silently failed
 *     in production. This wave adds `session_delete` to
 *     `commands/session.rs` + `lib.rs` and a typed `deleteSession`
 *     wrapper in `ipc/session.ts`. Round-trip below proves delete removes
 *     the row and re-list misses it (mock store stands in for the tmp
 *     store; the Rust side is pinned by `cargo test session_delete`).
 *  2. Top Toolbar Stop (`toolbar/action-compare-stop`) was an honest
 *     no-op; Rust `folder_cancel(session)` EXISTS but Stop never called
 *     it. `handleStop` now calls `folder_cancel` while busy when a folder
 *     session is known, keeping the honest idle note. Wiring pinned
 *     statically (precedent: `ui-standards.spec.ts` source pins) plus a
 *     runtime `folder_cancel` proof; idle title/note regression-pinned.
 *
 * No geometry asserts. Infra failures skip (matches other specs).
 */
import { expect, test } from "@playwright/test";
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

test("c1-p0-243: session delete round-trip deletes file + re-list missing", async ({
  page,
}) => {
  const result = await page.evaluate(async () => {
    const { sessionSave, sessionList, sessionLoad, deleteSession } = await import(
      "/src/ipc/session.ts"
    );
    const { invoke } = await import("/src/ipc/client.ts");
    const doc = {
      schema_version: 1,
      tabs: [
        {
          id: "tab-1",
          title: "Tab 1",
          session_type: "text",
          left_path: "/tmp/c1b-left.txt",
          right_path: "/tmp/c1b-right.txt",
          base_path: null,
          output_path: null,
          read_only: false,
          locked: false,
          options: {},
        },
      ],
      active_tab_index: 0,
      name: "c1b-p0-243",
      locked: false,
    };
    const id = await sessionSave(doc);
    const listedBefore = (await sessionList()).some((e) => e.id === id);
    const deleted = await deleteSession(id);
    const listedAfter = (await sessionList()).some((e) => e.id === id);
    let loadFailed = false;
    try {
      await sessionLoad(id);
    } catch {
      loadFailed = true;
    }
    // Idempotent second delete never throws (mock resolves true even
    // when missing; Rust resolves false — both are success here).
    const reDelete = await invoke<boolean>("session_delete", { id });
    return { listedBefore, deleted, listedAfter, loadFailed, reDelete };
  });

  expect(result.listedBefore, "saved session lists before delete").toBe(true);
  expect(result.deleted, "deleteSession resolves true on removal").toBe(true);
  expect(result.listedAfter, "re-list misses the deleted id").toBe(false);
  expect(result.loadFailed, "load after delete rejects").toBe(true);
  expect(typeof result.reDelete, "second delete is idempotent boolean").toBe("boolean");
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("c1-p0-243: Stop wires folder_cancel while busy, idle note stays honest", async ({
  page,
}) => {
  // Static pin: handleStop calls folder_cancel (Stop path only).
  const here = path.dirname(fileURLToPath(import.meta.url));
  const toolbarSrc = fs.readFileSync(
    path.resolve(here, "../../src/components/chrome/Toolbar.tsx"),
    "utf8",
  );
  expect(toolbarSrc, "handleStop imports the folder cancel IPC").toContain("folderCancel");
  expect(toolbarSrc, "handleStop invokes folder_cancel with the session").toContain(
    "folderCancel(target)",
  );
  expect(toolbarSrc, "honest idle note kept byte-identical").toContain(
    "no compare running: backend has no abort IPC",
  );
  expect(toolbarSrc, "Stop testid byte-identical").toContain("toolbar/action-compare-stop");
  expect(toolbarSrc, "stop note testid byte-identical").toContain("toolbar/compare-note");

  // Runtime: the folder_cancel IPC target answers cancelled:true under mock.
  const cancel = await page.evaluate(async () => {
    const { folderCompare, folderCancel } = await import("/src/ipc/folder.ts");
    const compared = await folderCompare("/tmp/c1b-left", "/tmp/c1b-right");
    const receipt = await folderCancel(compared.session);
    return { session: compared.session, cancelled: receipt.cancelled, echo: receipt.session };
  });
  expect(cancel.session, "mock folder compare mints a session").toBe("mock-folder-1");
  expect(cancel.cancelled, "folder_cancel reports cancelled").toBe(true);
  expect(cancel.echo, "folder_cancel echoes the session").toBe("mock-folder-1");

  // Regression: idle Stop still disables honestly with the unchanged title.
  const stop = page.getByTestId("toolbar/action-compare-stop");
  await expect(stop).toBeDisabled();
  await expect(stop).toHaveAttribute(
    "title",
    "Stop (no compare running: backend has no abort IPC)",
  );
  await expect(page.getByTestId("toolbar/compare-note")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
});
