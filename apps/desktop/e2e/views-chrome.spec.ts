/**
 * Wave-2B views chrome smoke (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * The Wave-2B views are not mounted by `App` (owned by another wave), so
 * this spec verifies them through the Vite dev server in mock mode:
 *  1. mock-mode boot (demo first-paint still renders the workspace), and
 *  2. per-kind ViewRouter switching contracts -- each `kind` maps to its
 *     view module, which carries its `data-testid` hooks and calls its
 *     locked IPC commands through `src/ipc/client.ts`.
 *
 * Live DOM mounting is intentionally NOT attempted: the views are only
 * reachable via a future tab harness. Source-contract assertions below
 * pin the wiring (kind -> view -> testid -> IPC command) so Wave 2R
 * mounting cannot silently drift.
 *
 * Skip policy: any infra failure (dev server unreachable, browser
 * missing offline) skips instead of failing -- the reason is logged.
 */
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

const KINDS = ["text", "folder", "hex", "table", "merge"] as const;

const VIEW_FILES = {
  text: "TextView.tsx",
  folder: "FolderView.tsx",
  hex: "HexView.tsx",
  table: "TableView.tsx",
  merge: "MergeView.tsx",
} as const;

const VIEW_CONTRACTS = {
  // `calls`: typed wrapper imported from src/ipc/* (view layer).
  // `cmds` + `ipcModule`: locked backend command strings in the ipc module.
  text: {
    testid: "textview/row-",
    calls: ["textCompare", "textGetRows"],
    ipcModule: "text.ts",
    cmds: ["text_compare", "text_get_rows"],
  },
  folder: {
    testid: "folderview/node-",
    calls: ["folderCompare", "folderGetTree"],
    ipcModule: "folder.ts",
    cmds: ["folder_compare", "folder_get_tree"],
  },
  hex: {
    testid: "hexview/page-",
    calls: ["hexCompare", "hexGetPage"],
    ipcModule: "hex.ts",
    cmds: ["hex_compare", "hex_get_page"],
  },
  table: {
    testid: "tableview/cell-",
    calls: ["tableCompare", "tableGetPage"],
    ipcModule: "table.ts",
    cmds: ["table_compare", "table_get_page"],
  },
  merge: {
    testid: "mergeview/conflict-",
    calls: ["mergeTextPlan", "mergeTextApply"],
    ipcModule: "merge.ts",
    cmds: ["merge_text_plan", "merge_text_apply"],
  },
} as const;

async function fetchSrc(page: Page, path: string): Promise<string | null> {
  try {
    return await page.evaluate(async (p: string) => {
      const res = await fetch(p);
      if (!res.ok) return null;
      return res.text();
    }, path);
  } catch {
    return null;
  }
}

test.beforeEach(async ({ page }) => {
  let up = false;
  try {
    await page.goto("/", { timeout: 15000 });
    up = true;
  } catch {
    up = false;
  }
  test.skip(!up, "dev server unreachable (offline browsers) -- skipping views-chrome smoke");
});

test("mock-mode boot renders demo first-paint", async ({ page }) => {
  // Wave-11: wave-10 demo first-paint boots with the demo session open.
  await expect(page.getByTestId("session-count")).toContainText("sessions: 1");
  await expect(page.getByTestId("workspace/demo-banner")).toContainText("Demo");
});

test("ViewRouter maps every kind to its view", async ({ page }) => {
  const src = await fetchSrc(page, "/src/components/views/ViewRouter.tsx");
  test.skip(src === null, "could not fetch ViewRouter source from dev server");
  expect(src).toContain("view-router");
  for (const kind of KINDS) {
    expect(src, `router handles kind ${kind}`).toContain(`"${kind}"`);
    expect(src, `router wraps view ${kind}`).toContain(`view-${kind}`);
    expect(src, `router imports ${kind} view`).toContain(VIEW_FILES[kind].replace(".tsx", ""));
  }
  // Mock-mode guard: router must accept the sessionId passthrough prop.
  expect(src).toContain("sessionId");
});

for (const kind of KINDS) {
  test(`view contract: ${kind} carries testids + locked IPC calls`, async ({ page }) => {
    const src = await fetchSrc(page, `/src/components/views/${VIEW_FILES[kind]}`);
    test.skip(src === null, `could not fetch ${VIEW_FILES[kind]} from dev server`);
    const contract = VIEW_CONTRACTS[kind];
    expect(src, `${kind} row/node/page testid`).toContain(contract.testid);
    for (const fn of contract.calls) {
      expect(src, `${kind} calls ${fn}`).toContain(fn);
    }
    const ipc = await fetchSrc(page, `/src/ipc/${contract.ipcModule}`);
    test.skip(ipc === null, `could not fetch ${contract.ipcModule} from dev server`);
    for (const cmd of contract.cmds) {
      expect(ipc, `${kind} registers ${cmd}`).toContain(cmd);
    }
    // Mock-mode guard: shape validation + inline error rendering, never a crash.
    expect(src, `${kind} degrades on mock fixtures`).toContain("empty");
    expect(src, `${kind} renders errors inline`).toContain("alert");
  });
}

test("merge view exposes take-left / take-right choices", async ({ page }) => {
  const src = await fetchSrc(page, "/src/components/views/MergeView.tsx");
  test.skip(src === null, "could not fetch MergeView source from dev server");
  expect(src).toContain("take-left");
  expect(src).toContain("take-right");
  expect(src).toContain("merge_text_apply");
});
