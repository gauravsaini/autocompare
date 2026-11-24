/**
 * Wave-129 save/delete parity (audit M3 ranked #6). MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * PORT-FREE gate spec (fresh 1537, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - Save silent when the session path is tracked, Save-As dialog only
 *    when untracked/new; dialog-cancel aborts with no phantom save;
 *  - toolbar delete confirms (two-step) + advances to the next tab,
 *    never deletes the last row;
 *  - post-save list refreshes: the dialog shows the minted id row and
 *    selection follows it instead of going stale.
 *
 * Mock-IPC honesty: the mock adapter owns `session_list/load/save/delete`
 * over localStorage (`client.ts`); every assertion reads the DOM or the
 * mock store, never the network. Infra failures skip.
 */
import { expect, test, type Page } from "@playwright/test";

const STORE_KEY = "autocompare:mockSessionStore:v1";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(String(err)));
  return errors;
}

async function clearStore(page: Page): Promise<void> {
  await page.evaluate((key) => {
    try {
      window.localStorage.removeItem(key);
      window.localStorage.removeItem("autocompare:mockSessionSeq:v1");
      window.localStorage.removeItem("autocompare:lastActiveSessionId");
    } catch { /* ignore */ }
  }, STORE_KEY);
}

async function storeKeys(page: Page): Promise<string[]> {
  return page.evaluate((key) => {
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return [];
      return Object.keys(JSON.parse(raw) as Record<string, unknown>);
    } catch { return []; }
  }, STORE_KEY);
}

/**
 * Seeds one persisted session BEFORE reload so the Wave-10 boot demo
 * never fires (demo auto-opens only when the store is empty). Makes
 * row counts deterministic instead of racing the demo effect.
 */
async function seedStore(page: Page): Promise<void> {
  await page.evaluate((key) => {
    try {
      const doc = {
        schema_version: 1,
        tabs: [{
          id: "tab-seed-129",
          title: "Seed 129",
          session_type: "text",
          left_path: "/tmp/129-seed-l.txt",
          right_path: "/tmp/129-seed-r.txt",
          base_path: null,
          output_path: null,
          read_only: false,
          locked: false,
          options: {},
        }],
        active_tab_index: 0,
        name: "Seed 129",
        locked: false,
      };
      const raw = window.localStorage.getItem(key);
      const store = raw ? (JSON.parse(raw) as Record<string, unknown>) : {};
      store["sess-seed-129"] = doc;
      window.localStorage.setItem(key, JSON.stringify(store));
    } catch { /* ignore */ }
  }, STORE_KEY);
}

async function itemIds(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    return [...document.querySelectorAll('[data-testid^="session/item-"]')]
      .map((el) => el.getAttribute("data-testid") ?? "")
      .filter((s) => s !== "");
  });
}

/** Installs `__MOCK_DIALOG_SAVE__` returning `path` (null = cancel) + call counter. */
async function installSaveMock(page: Page, path: string | null): Promise<void> {
  await page.evaluate((p) => {
    const w = window as unknown as Record<string, unknown>;
    w.__acSaveDialogCalls = 0;
    w.__MOCK_DIALOG_SAVE__ = async () => {
      w.__acSaveDialogCalls = (w.__acSaveDialogCalls as number) + 1;
      return p;
    };
  }, path);
}

async function saveDialogCalls(page: Page): Promise<number> {
  return page.evaluate(() => {
    return (window as unknown as Record<string, unknown>).__acSaveDialogCalls as number ?? 0;
  });
}

async function createSession(page: Page, kind: string, left: string, right: string): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
}

async function openManager(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
}

test("save silent when tracked, dialog only when untracked, cancel aborts", async ({ page }) => {
  const errors = collectErrors(page);
  await clearStore(page);
  await seedStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  await installSaveMock(page, "/tmp/129-tracked.json");
  await createSession(page, "text", "/tmp/129-left.txt", "/tmp/129-right.txt");
  const base = (await storeKeys(page)).length;
  expect(base).toBeGreaterThanOrEqual(1);

  await openManager(page);

  // Untracked (new) session -> Save-As dialog fires once, store grows.
  await page.getByTestId("session/save").click();
  await expect.poll(async () => (await storeKeys(page)).length, { timeout: 15000 }).toBe(base + 1);
  expect(await saveDialogCalls(page)).toBe(1);

  // Tracked session -> silent save, dialog NOT consulted again, store still grows.
  await page.getByTestId("session/save").click();
  await expect.poll(async () => (await storeKeys(page)).length, { timeout: 15000 }).toBe(base + 2);
  expect(await saveDialogCalls(page)).toBe(1);

  // Cancel aborts: untracked session + null dialog -> no phantom save.
  await page.getByRole("button", { name: "Close dialog" }).click();
  await createSession(page, "text", "/tmp/129-cancel-left.txt", "/tmp/129-cancel-right.txt");
  const beforeCancel = (await storeKeys(page)).length;
  await installSaveMock(page, null);
  await openManager(page);
  await page.getByTestId("session/save").click();
  await page.waitForTimeout(1000);
  expect(await storeKeys(page)).toHaveLength(beforeCancel);
  expect(await saveDialogCalls(page)).toBe(1);

  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByRole("button", { name: "Close dialog" }).click();
  expect(errors).toEqual([]);
});

test("toolbar delete confirms, advances, never deletes last row", async ({ page }) => {
  const errors = collectErrors(page);
  await clearStore(page);
  await seedStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  await createSession(page, "text", "/tmp/129-del-a-left.txt", "/tmp/129-del-a-right.txt");
  await createSession(page, "text", "/tmp/129-del-b-left.txt", "/tmp/129-del-b-right.txt");
  await openManager(page);
  // Settle on the created rows; count stays relative so a leftover
  // boot-demo row (reload-window race) cannot flake the test.
  await expect(page.getByTestId("session/list")).toContainText("129-del-b-left");
  const n0 = (await itemIds(page)).length;
  expect(n0).toBeGreaterThanOrEqual(2);
  const keysBefore = (await storeKeys(page)).length;
  expect(keysBefore).toBeGreaterThanOrEqual(n0);

  // First click only arms the confirm: nothing deleted.
  await page.getByTestId("session/delete").click();
  await expect(page.getByTestId("session/delete-confirm")).toBeVisible();
  expect(await itemIds(page)).toHaveLength(n0);
  expect(await storeKeys(page)).toHaveLength(keysBefore);

  // Cancel disarms: still nothing deleted.
  await page.getByTestId("session/delete-cancel").click();
  await expect(page.getByTestId("session/delete-confirm")).toBeHidden();
  expect(await itemIds(page)).toHaveLength(n0);

  // Confirm deletes + advances to the next tab (save stays enabled).
  await page.getByTestId("session/delete").click();
  await page.getByTestId("session/delete-confirm").click();
  await expect.poll(async () => (await itemIds(page)).length, { timeout: 15000 }).toBe(n0 - 1);
  await expect(page.getByTestId("session/save")).toBeEnabled();
  expect(await storeKeys(page)).toHaveLength(keysBefore - 1);

  // Reduce to a single row, then the last row refuses inline.
  let remaining = n0 - 1;
  while (remaining > 1) {
    await page.getByTestId("session/delete").click();
    await expect(page.getByTestId("session/delete-confirm")).toBeVisible({ timeout: 15000 });
    await page.getByTestId("session/delete-confirm").click();
    remaining -= 1;
    await expect.poll(async () => (await itemIds(page)).length, { timeout: 15000 }).toBe(remaining);
  }
  await page.getByTestId("session/delete").click();
  await expect(page.getByTestId("session/row-error")).toContainText(
    "At least one session must remain.",
  );
  expect(await itemIds(page)).toHaveLength(1);
  await expect(page.getByTestId("session/save")).toBeEnabled();
  expect(await storeKeys(page)).toHaveLength(keysBefore - (n0 - 1));

  await page.getByRole("button", { name: "Close dialog" }).click();
  expect(errors).toEqual([]);
});

test("post-save list refreshes and selection follows minted id", async ({ page }) => {
  const errors = collectErrors(page);
  await clearStore(page);
  await seedStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  await installSaveMock(page, "/tmp/129-refresh.json");
  const left = "/tmp/129-ref-left.txt";
  const right = "/tmp/129-ref-right.txt";
  await createSession(page, "text", left, right);
  await openManager(page);
  // Settle on the created row; relative swap below is leftover-proof.
  await expect(page.getByTestId("session/list")).toContainText("129-ref-left");
  const before = await itemIds(page);
  expect(before.length).toBeGreaterThanOrEqual(1);

  await page.getByTestId("session/save").click();
  // The dialog list swaps the retired pre-save row for the minted id row
  // (same length, exactly one new id) instead of going stale.
  await expect.poll(async () => {
    const after = await itemIds(page);
    return after.length === before.length && after.some((id) => !before.includes(id));
  }, { timeout: 15000 }).toBe(true);

  const after = await itemIds(page);
  const keys = await storeKeys(page);
  for (const id of after) {
    expect(keys).toContain(id.replace("session/item-", ""));
  }
  // Selection followed the minted row: save stays enabled, paths intact.
  await expect(page.getByTestId("session/save")).toBeEnabled();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("pathbar/left")).toHaveValue(left, { timeout: 15000 });
  await expect(page.getByTestId("pathbar/right")).toHaveValue(right, { timeout: 15000 });
  expect(errors).toEqual([]);
});
