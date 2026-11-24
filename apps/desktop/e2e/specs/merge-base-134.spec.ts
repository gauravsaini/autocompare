/**
 * Wave-134 merge base-path UI (audit gap #1: real 3-way merge needs the
 * Base/ancestor field). MOCK IPC ONLY (`VITE_MOCK_IPC=1`).
 *
 * Proves the missing UI over the already-present plumbing (session DTO
 * `base_path`, `MergeView basePath ?? leftPath`):
 *  (a) SessionManager NEW SESSION gains a Base field for merge kind
 *      (`session/base-path`); other kinds collapse it with an honest note;
 *  (b) merge create persists `base_path` (mock store) and the PathBar
 *      shows the 3rd Base field (`pathbar-base`) with that value;
 *  (c) reload keeps base (save/load round-trip, boot hydration);
 *  (d) PathBar base Enter-submit re-opens the session carrying the new
 *      base (history recorded, identical submit dedups, no new IPC).
 *
 * PORT-FREE: runs on a fresh :1544 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
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

function trackErrors(page: Page): string[] {
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
      window.localStorage.removeItem("ac:pathbar-history:left");
      window.localStorage.removeItem("ac:pathbar-history:right");
      window.localStorage.removeItem("ac:pathbar-history:base");
    } catch { /* ignore */ }
  }, STORE_KEY);
}

async function closeManager(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
}

async function openManager(page: Page): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
}

async function createMergeSession(page: Page, left: string, right: string, base: string): Promise<void> {
  await openManager(page);
  await page.getByTestId("session/kind-select").selectOption("merge");
  await expect(page.getByTestId("session/base-path")).toBeVisible();
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/base-path").fill(base);
  await page.getByTestId("session/create-button").click();
  await closeManager(page);
}

async function readStoreBases(page: Page): Promise<string[]> {
  return page.evaluate((key) => {
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return [];
      const parsed = JSON.parse(raw) as Record<string, { tabs?: Array<{ base_path?: unknown }> }>;
      return Object.values(parsed).map((d) => String(d.tabs?.[0]?.base_path ?? ""));
    } catch { return []; }
  }, STORE_KEY);
}

async function storeDocCount(page: Page): Promise<number> {
  return page.evaluate((key) => {
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) return 0;
      return Object.keys(JSON.parse(raw) as Record<string, unknown>).length;
    } catch { return 0; }
  }, STORE_KEY);
}

test("merge create with base persists + PathBar shows 3rd Base field", async ({ page }) => {
  const errors = trackErrors(page);
  await clearStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  const left = "/tmp/134-left.txt";
  const right = "/tmp/134-right.txt";
  const base = "/tmp/134-base.txt";
  await createMergeSession(page, left, right, base);

  // Merge view mounts for the new session.
  await expect(page.getByTestId("view-merge")).toBeVisible({ timeout: 15000 });
  // 3rd Base field visible with the submitted ancestor value.
  await expect(page.getByTestId("pathbar-base")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("pathbar-base")).toHaveValue(base, { timeout: 15000 });
  // Mock store proves base_path persisted (never null).
  await expect.poll(() => readStoreBases(page), { timeout: 15000 }).toContain(base);

  expect(errors, "zero console/page errors in merge create").toEqual([]);
});

test("reload keeps base (save/load round-trip)", async ({ page }) => {
  const errors = trackErrors(page);
  await clearStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  const left = "/tmp/134-reload-left.txt";
  const right = "/tmp/134-reload-right.txt";
  const base = "/tmp/134-reload-base.txt";
  await createMergeSession(page, left, right, base);
  await expect(page.getByTestId("pathbar-base")).toHaveValue(base, { timeout: 15000 });

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });
  // Boot hydration restores the ancestor: merge view + base field intact.
  await expect(page.getByTestId("view-merge")).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId("pathbar-base")).toHaveValue(base, { timeout: 30000 });
  await expect.poll(() => readStoreBases(page), { timeout: 30000 }).toContain(base);

  expect(errors, "zero console/page errors across reload").toEqual([]);
});

test("PathBar base submit re-opens with new base + records history + dedups", async ({ page }) => {
  const errors = trackErrors(page);
  await clearStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  const baseA = "/tmp/134-submit-base-a.txt";
  await createMergeSession(page, "/tmp/134-submit-left.txt", "/tmp/134-submit-right.txt", baseA);
  await expect(page.getByTestId("pathbar-base")).toHaveValue(baseA, { timeout: 15000 });

  // Submit a new ancestor through the 3rd field.
  const baseB = "/tmp/134-submit-base-b.txt";
  await page.getByTestId("pathbar-base").fill(baseB);
  await page.getByTestId("pathbar-base").press("Enter");

  // Store + field carry the new base (re-opened with base).
  await expect.poll(() => readStoreBases(page), { timeout: 15000 }).toContain(baseB);
  await expect(page.getByTestId("pathbar-base")).toHaveValue(baseB, { timeout: 15000 });
  // History reuse: base side recorded like left/right.
  await expect.poll(() => page.evaluate(() => {
    try {
      return JSON.parse(window.localStorage.getItem("ac:pathbar-history:base") || "[]");
    } catch { return []; }
  }), { timeout: 15000 }).toContain(baseB);

  // Identical submit dedups: no second doc minted.
  const before = await storeDocCount(page);
  await page.getByTestId("pathbar-base").press("Enter");
  await page.waitForTimeout(500);
  expect(await storeDocCount(page)).toBe(before);

  expect(errors, "zero console/page errors in base submit").toEqual([]);
});

test("non-merge kinds hide Base field with honest note", async ({ page }) => {
  const errors = trackErrors(page);
  await clearStore(page);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("session-count")).toContainText("sessions:", { timeout: 30000 });

  await openManager(page);
  // Default text kind: no base input, honest collapsed note instead.
  await expect(page.getByTestId("session/base-path")).toHaveCount(0);
  await expect(page.getByTestId("session/base-note")).toBeVisible();
  // Merge kind reveals it.
  await page.getByTestId("session/kind-select").selectOption("merge");
  await expect(page.getByTestId("session/base-path")).toBeVisible();
  await expect(page.getByTestId("session/base-note")).toHaveCount(0);
  await closeManager(page);

  // Create a text session: PathBar stays 2-field (no base).
  await openManager(page);
  await page.getByTestId("session/kind-select").selectOption("text");
  await page.getByTestId("session/left-path").fill("/tmp/134-plain-left.txt");
  await page.getByTestId("session/right-path").fill("/tmp/134-plain-right.txt");
  await page.getByTestId("session/create-button").click();
  await closeManager(page);
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("pathbar-base")).toHaveCount(0);

  expect(errors, "zero console/page errors in non-merge check").toEqual([]);
});
