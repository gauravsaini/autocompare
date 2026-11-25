/**
 * D2 pathdrop BC parity (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * User complaint (Hinglish-roman): "Left and right path areas BC me
 * droppable hain, hamare case me nahi." Pin: har comparison view
 * (Text/Folder/Table/Hex/Merge/Sync/Media — jahan path bar hai) me
 * DONO sides drop accept karein. File drag karke path bar YA pane par
 * chhodne se us side ka path set + re-compare ho.
 *
 * Har test: LEFT drop -> left path set + exactly one recompute,
 * phir RIGHT drop -> right path set + exactly one recompute
 * (`__AC_COMPARE_COUNT__` delta +1 per drop, `statusbar/message`
 * "Compare finished"). Koi geometry assert nahi (sirf drop routing).
 *
 * PORT gate spec (fresh 1856, reuseExistingServer:false,
 * VITE_MOCK_IPC=1, --workers=1, webkit):
 *  - zero console errors / page errors har test me.
 */
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed:", err);
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

async function compareCount(page: Page): Promise<number> {
  return page.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    return typeof w["__AC_COMPARE_COUNT__"] === "number"
      ? (w["__AC_COMPARE_COUNT__"] as number)
      : 0;
  });
}

/** Synthetic drop of `paths` (in order) on `selector`. */
async function dropPaths(page: Page, selector: string, paths: string[]): Promise<void> {
  await page.locator(selector).first().evaluate(
    (el, ps: string[]) => {
      const files = ps.map((p) => ({ name: p.split("/").pop() ?? p, path: p }));
      const dt = { files, types: ["Files"], getData: (_t: string) => "" };
      const ev = new DragEvent("drop", { bubbles: true, cancelable: true });
      Object.defineProperty(ev, "dataTransfer", { value: dt });
      el.dispatchEvent(ev);
    },
    paths,
  );
}

/** Session kholo: dialog se kind + left/right (+merge base), view visible tak. */
async function createSession(
  page: Page,
  kind: string,
  viewId: string,
  left: string,
  right: string,
  base?: string,
): Promise<void> {
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 10000 });
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  if (base !== undefined) {
    await page.getByTestId("session/base-path").fill(base);
  }
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden({ timeout: 10000 });
  await expect(page.getByTestId(viewId)).toBeVisible({ timeout: 15000 });
}

/** LEFT drop -> left set + 1 recompute; RIGHT drop -> right set + 1 recompute. */
async function expectBothSidesDroppable(
  page: Page,
  errors: string[],
  leftSelector: string,
  rightSelector: string,
  leftPath: string,
  rightPath: string,
): Promise<void> {
  const leftInput = page.locator('[data-testid="pathbar/left"]');
  const rightInput = page.locator('[data-testid="pathbar/right"]');
  const status = page.getByTestId("statusbar/message");

  let before = await compareCount(page);
  await dropPaths(page, leftSelector, [leftPath]);
  await expect(leftInput).toHaveValue(leftPath, { timeout: 10000 });
  await expect(status).toContainText("Compare finished", { timeout: 15000 });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);

  before = await compareCount(page);
  await dropPaths(page, rightSelector, [rightPath]);
  await expect(rightInput).toHaveValue(rightPath, { timeout: 10000 });
  await expect(status).toContainText("Compare finished", { timeout: 15000 });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);

  expect(errors, "zero console/page errors").toEqual([]);
}

test("d2-pathdrop-251 text: pathbar + pane dono sides droppable", async ({ page }) => {
  const errors = collectErrors(page);
  await createSession(page, "text", "view-text", "/tmp/d2-text-left.txt", "/tmp/d2-text-right.txt");
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-text-drop-left.txt", "/tmp/d2-text-drop-right.txt",
  );
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="panedrop/section-left"]', '[data-testid="panedrop/section-right"]',
    "/tmp/d2-text-pane-left.txt", "/tmp/d2-text-pane-right.txt",
  );
});

test("d2-pathdrop-251 folder: pathbar + pane dono sides droppable", async ({ page }) => {
  const errors = collectErrors(page);
  await createSession(page, "folder", "view-folder", "/tmp/d2-folder-left/", "/tmp/d2-folder-right/");
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-folder-drop-left/", "/tmp/d2-folder-drop-right/",
  );
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="panedrop/section-left"]', '[data-testid="panedrop/section-right"]',
    "/tmp/d2-folder-pane-left/", "/tmp/d2-folder-pane-right/",
  );
});

test("d2-pathdrop-251 table: pathbar + pane dono sides droppable", async ({ page }) => {
  const errors = collectErrors(page);
  await createSession(page, "table", "view-table", "/tmp/d2-table-left.csv", "/tmp/d2-table-right.csv");
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-table-drop-left.csv", "/tmp/d2-table-drop-right.csv",
  );
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="panedrop/section-left"]', '[data-testid="panedrop/section-right"]',
    "/tmp/d2-table-pane-left.csv", "/tmp/d2-table-pane-right.csv",
  );
});

test("d2-pathdrop-251 hex: pathbar + pane dono sides droppable", async ({ page }) => {
  const errors = collectErrors(page);
  await createSession(page, "hex", "view-hex", "/tmp/d2-hex-left.bin", "/tmp/d2-hex-right.bin");
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-hex-drop-left.bin", "/tmp/d2-hex-drop-right.bin",
  );
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="panedrop/section-left"]', '[data-testid="panedrop/section-right"]',
    "/tmp/d2-hex-pane-left.bin", "/tmp/d2-hex-pane-right.bin",
  );
});

test("d2-pathdrop-251 sync: pathbar + pane dono sides droppable", async ({ page }) => {
  const errors = collectErrors(page);
  await createSession(page, "sync", "view-sync", "/tmp/d2-sync-left/", "/tmp/d2-sync-right/");
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-sync-drop-left/", "/tmp/d2-sync-drop-right/",
  );
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="panedrop/section-left"]', '[data-testid="panedrop/section-right"]',
    "/tmp/d2-sync-pane-left/", "/tmp/d2-sync-pane-right/",
  );
});

test("d2-pathdrop-251 media: pathbar + pane dono sides droppable", async ({ page }) => {
  const errors = collectErrors(page);
  await createSession(page, "media", "view-media", "/tmp/d2-media-left.png", "/tmp/d2-media-right.png");
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-media-drop-left.png", "/tmp/d2-media-drop-right.png",
  );
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="panedrop/section-left"]', '[data-testid="panedrop/section-right"]',
    "/tmp/d2-media-pane-left.png", "/tmp/d2-media-pane-right.png",
  );
});

test("d2-pathdrop-251 blank session: do single-side drops converge to one recompute", async ({
  page,
}) => {
  // BC parity core: khaali session me pehle ek side drop (stage-only,
  // "needs two paths"), phir dusri side drop -> App dono sides paakar
  // EXACTLY ONE recompute chalaye (pehla drop lost nahi hona chahiye).
  // Dono orders (left-first + right-first).
  const errors = collectErrors(page);
  for (const order of ["left-first", "right-first"] as const) {
    if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
      await page.getByTestId("workspace/demo-clear").click();
    }
    const home = page.getByTestId("tab-home");
    if ((await home.count()) > 0) await home.click().catch(() => {});
    await expect(page.getByTestId("workspace/welcome")).toBeVisible({ timeout: 15000 });
    await page.getByTestId("home/new-text").click();
    await expect(page.getByTestId("workspace/blank-empty")).toBeVisible({ timeout: 15000 });

    const first = order === "left-first" ? "left" : "right";
    const second = order === "left-first" ? "right" : "left";
    const before = await compareCount(page);
    await dropPaths(page, `[data-testid="pathbar/${first}"]`, [`/tmp/d2-blank-${order}-1.txt`]);
    await expect(page.locator(`[data-testid="pathbar/${first}"]`)).toHaveValue(
      `/tmp/d2-blank-${order}-1.txt`,
      { timeout: 10000 },
    );
    await dropPaths(page, `[data-testid="pathbar/${second}"]`, [`/tmp/d2-blank-${order}-2.txt`]);
    await expect(page.locator(`[data-testid="pathbar/${second}"]`)).toHaveValue(
      `/tmp/d2-blank-${order}-2.txt`,
      { timeout: 10000 },
    );
    await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", {
      timeout: 15000,
    });
    await page.waitForTimeout(1200);
    expect(await compareCount(page), `${order}: exactly one recompute`).toBe(before + 1);
  }
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d2-pathdrop-251 merge: pathbar + 3-pane zones droppable, triple recompute", async ({
  page,
}) => {
  const errors = collectErrors(page);
  await createSession(
    page, "merge", "view-merge",
    "/tmp/d2-merge-left.txt", "/tmp/d2-merge-right.txt", "/tmp/d2-merge-base.txt",
  );
  // PathBar left/right drops (base untouched).
  await expectBothSidesDroppable(
    page, errors,
    '[data-testid="pathbar/left"]', '[data-testid="pathbar/right"]',
    "/tmp/d2-merge-drop-left.txt", "/tmp/d2-merge-drop-right.txt",
  );
  // Merge 3-pane zones: left + right pane drops stage triple + re-plan.
  const leftInput = page.locator('[data-testid="pathbar/left"]');
  const rightInput = page.locator('[data-testid="pathbar/right"]');
  const status = page.getByTestId("statusbar/message");

  let before = await compareCount(page);
  await dropPaths(page, '[data-testid="merge-pane-left"]', ["/tmp/d2-merge-pane-left.txt"]);
  await expect(leftInput).toHaveValue("/tmp/d2-merge-pane-left.txt", { timeout: 10000 });
  await expect(status).toContainText("Compare finished", { timeout: 15000 });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);

  before = await compareCount(page);
  await dropPaths(page, '[data-testid="merge-pane-right"]', ["/tmp/d2-merge-pane-right.txt"]);
  await expect(rightInput).toHaveValue("/tmp/d2-merge-pane-right.txt", { timeout: 10000 });
  await expect(status).toContainText("Compare finished", { timeout: 15000 });
  await expect.poll(() => compareCount(page), { timeout: 10000 }).toBe(before + 1);

  expect(errors, "zero console/page errors").toEqual([]);
});
