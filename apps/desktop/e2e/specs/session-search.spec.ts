/**
 * Wave-40 session picker filter proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Drives the REAL `SessionManager` dialog (via the `mountSessionManager`
 * test helper, same as `session-actions.spec.ts`) with three controlled
 * sessions and proves Beyond Compare style live filtering:
 *  - typing narrows rows live (case-insensitive substring across
 *    name/kind/paths; paths arrive embedded in `name` under the locked
 *    `CachedSession` contract, e.g. "text: a.txt <-> b.txt");
 *  - the header count + status line stay honest ("1 of 3 shown");
 *  - a query with no hits renders the empty state with a working
 *    clear button;
 *  - Escape clears the query and restores all rows;
 *  - selection (`activeId`) survives filtering (hidden rows stay active);
 *  - wave-25 row actions keep working on filtered rows (favorite star);
 *  - the filter input is keyboard reachable;
 *  - light + dark PNGs land in `/tmp/ui-40/` (never in-repo);
 *  - zero console errors / page errors throughout.
 *
 * Mock-IPC honesty: every assertion reads the DOM, never the network.
 * Skip policy matches the other specs (infra failures skip).
 * NEW file only; no existing spec touched.
 */
import { expect, test, type Page } from "@playwright/test";

const EVID_DIR = "/tmp/ui-40";

const SEED = [
  {
    id: "sess-alpha",
    kind: "text",
    name: "Alpha notes: /docs/a.txt ↔ /docs/b.txt",
    locked: false,
    pageCount: 1,
  },
  {
    id: "sess-beta",
    kind: "folder",
    name: "Beta sync: /data/left ↔ /data/right",
    locked: false,
    pageCount: 2,
  },
  {
    id: "sess-gamma",
    kind: "table",
    name: "Gamma Report",
    locked: false,
    pageCount: 1,
  },
];

async function mountManager(
  page: Page,
  sessions = SEED,
  opts: { readonly pinActive?: boolean } = {},
) {
  const pinActive = opts.pinActive ?? true;
  await page.evaluate(
    async ({ seed, pin }) => {
      const { mountSessionManager } = await import(
        "/src/components/session/index.ts"
      );
      let container = document.getElementById("session-test-container");
      if (!container) {
        container = document.createElement("div");
        container.id = "session-test-container";
        document.body.appendChild(container);
      }
      (window as unknown as Record<string, unknown>).__acClosed = false;
      mountSessionManager(container, {
        isOpen: true,
        sessions: seed,
        // Pinned activeId mirrors how App hosts the dialog; omitted (via
        // pin:false) the dialog owns selection through the hook. The
        // selection-preservation step needs the unpinned form because the
        // pinned prop intentionally wins over clicks on every hook change.
        ...(pin ? { activeId: seed[0]?.id ?? null } : {}),
        onClose: () => {
          (window as unknown as Record<string, unknown>).__acClosed = true;
        },
      });
    },
    { seed: sessions, pin: pinActive },
  );
}

function itemIds(page: Page) {
  return page
    .getByTestId("session/list")
    .locator('[data-testid^="session/item-"]');
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({
      timeout: 15000,
    });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("session search filters live, empties honestly, Escape clears, actions survive", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  await mountManager(page);
  const dialog = page.getByTestId("session/dialog");
  await expect(dialog).toBeVisible();

  const search = page.getByTestId("session/search");
  // Keyboard reachable: tab to it or focus it, then type.
  await search.focus();
  await expect(search).toBeFocused();

  // Baseline: all three rows + honest full count.
  await expect(itemIds(page)).toHaveCount(3);
  await expect(page.getByTestId("session/count")).toContainText("3 sessions");

  // --- live narrowing by name (case-insensitive) ---
  await search.fill("alpha");
  await expect(itemIds(page)).toHaveCount(1);
  await expect(page.getByTestId("session/item-sess-alpha")).toBeVisible();
  await expect(page.getByTestId("session/count")).toContainText("1 of 3");
  await expect(page.getByTestId("session/search-status")).toContainText(
    "1 of 3 shown",
  );

  // --- case-insensitive kind match ---
  await search.fill("FOLDER");
  await expect(itemIds(page)).toHaveCount(1);
  await expect(page.getByTestId("session/item-sess-beta")).toBeVisible();

  // --- path substring match (paths embedded in the row name) ---
  await search.fill("/data/left");
  await expect(itemIds(page)).toHaveCount(1);
  await expect(page.getByTestId("session/item-sess-beta")).toBeVisible();

  // --- kind substring shared by one row ---
  await search.fill("table");
  await expect(itemIds(page)).toHaveCount(1);
  await expect(page.getByTestId("session/item-sess-gamma")).toBeVisible();

  // --- row actions keep working on a filtered row (favorite star) ---
  const star = page.getByTestId("session/favorite-sess-gamma");
  await expect(star).toHaveAttribute("aria-pressed", "false");
  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "true");

  // --- selection survives filtering: select beta, filter it out, clear ---
  // Remount unpinned so clicks own the selection (a pinned activeId prop
  // intentionally wins over clicks on every hook change).
  await mountManager(page, SEED, { pinActive: false });
  const search2 = page.getByTestId("session/search");
  await search2.fill("");
  await expect(itemIds(page)).toHaveCount(3);
  await page.getByTestId("session/item-sess-beta").click();
  await expect(dialog).toContainText("Beta sync");
  await search2.fill("gamma");
  await expect(itemIds(page)).toHaveCount(1);
  // Active session label still names the hidden-but-selected row.
  await expect(dialog).toContainText("Beta sync");
  await search2.fill("");
  await expect(itemIds(page)).toHaveCount(3);
  await expect(dialog).toContainText("Beta sync");

  // --- empty result: honest empty state with a working clear button ---
  await search.fill("zzz-no-such-session");
  await expect(itemIds(page)).toHaveCount(0);
  await expect(page.getByTestId("session/search-empty")).toContainText(
    "No sessions match",
  );
  await expect(page.getByTestId("session/count")).toContainText("0 of 3");
  await page.getByTestId("session/search-clear-empty").click();
  await expect(itemIds(page)).toHaveCount(3);
  await expect(page.getByTestId("session/count")).toContainText("3 sessions");

  // --- Escape clears the query ---
  await search.fill("alpha");
  await expect(itemIds(page)).toHaveCount(1);
  await search.press("Escape");
  await expect(itemIds(page)).toHaveCount(3);
  await expect(search).toHaveValue("");

  // --- header X clears too ---
  await search.fill("beta");
  await expect(itemIds(page)).toHaveCount(1);
  await page.getByTestId("session/search-clear").click();
  await expect(itemIds(page)).toHaveCount(3);

  // Light evidence with an active filter applied.
  await search.fill("alpha");
  await expect(itemIds(page)).toHaveCount(1);
  await page.screenshot({ path: `${EVID_DIR}/session-search-light.png` });
  await search.fill("");

  // --- dark evidence: remount and shoot under dark color scheme ---
  await page.emulateMedia({ colorScheme: "dark" });
  await mountManager(page);
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/search").fill("beta");
  await expect(itemIds(page)).toHaveCount(1);
  await page.screenshot({ path: `${EVID_DIR}/session-search-dark.png` });
  await page.emulateMedia({ colorScheme: "light" });

  expect(consoleErrors).toEqual([]);
  expect(pageErrors).toEqual([]);
});
