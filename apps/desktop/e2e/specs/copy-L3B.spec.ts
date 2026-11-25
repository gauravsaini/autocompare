/**
 * Wave-L3B copy bus wiring (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Proves the BC unified-toolbar parity slice: App `handleViewCommand`
 * ("copy"/"copy-right") forwards per active kind via the view bus, and
 * each view owns its follow-through over EXISTING local handlers (no
 * engine change, no new IPC):
 *  - table routes bus copy -> copyCell, bus copy-right -> copyRow
 *    (existing handlers, `__TABLE_COPY_LAST__` headless hook);
 *  - folder routes bus copy/copy-right -> copyPath of the selected
 *    file row (existing handler, `__FOLDER_COPY_LAST__` hook);
 *  - hex routes bus copy -> copyHex, bus copy-right -> copyAscii
 *    (existing handlers, `hex-copy-status` receipt);
 *  - sync parks honestly in-view (no per-file copy affordance:
 *    `__SYNC_COPY_PARKED__` hook, plan untouched);
 *  - media parks honestly in App (summary view: score + metrics, no
 *    cursor — no dispatch, parked status note, never a crash).
 *
 * Top-bar buttons stay disabled with the parked title outside
 * text/merge (toolbar enablement belongs to a later wave — this spec
 * pins NO visual change, only bus-level wiring: the App forward is
 * driven through the `__MOCK_EMIT_MENU__` session.copy path, the same
 * `handleViewCommand("copy")` the toolbar calls). Skip policy matches
 * the other specs (infra failures skip). New file only.
 */
import { expect, test, type Page } from "@playwright/test";

test.setTimeout(120_000);

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function trackErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewTestid: string,
): Promise<void> {
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewTestid)).toBeVisible({ timeout: 15000 });
}

/** Drive the App `handleViewCommand("copy")` path (toolbar/menu parity). */
async function appCopy(page: Page): Promise<void> {
  await page.evaluate(() => {
    (window as unknown as { __MOCK_EMIT_MENU__?: (p: { id: string }) => void }).__MOCK_EMIT_MENU__?.({
      id: "session.copy",
    });
  });
}

/** Drive the raw view bus (view-level follow-through + isolation). */
async function busCopy(page: Page, cmd: "copy" | "copy-right", viewKind: string): Promise<void> {
  await page.evaluate(
    ({ c, k }) => {
      window.dispatchEvent(
        new CustomEvent("autocompare:view-command", {
          detail: { cmd: c, viewKind: k, dir: c === "copy-right" ? "right" : "left" },
        }),
      );
    },
    { c: cmd, k: viewKind },
  );
}

test("L3B table: App copy forwards on bus (copyCell/copyRow), foreign kind isolated", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "table", "/tmp/l3b-table-left.csv", "/tmp/l3b-table-right.csv", "view-table");
  await expect(page.getByTestId("tableview/row-0")).toBeVisible({ timeout: 15000 });

  // Top bar stays parked (NO visual change this wave).
  await expect(page.getByTestId("toolbar/action-copy")).toBeDisabled();

  // Select a cell, then drive the App copy path: status + cell copy hook.
  await page.getByTestId("tableview/cell-0-0").click();
  await expect(page.locator('[data-selected="true"]')).toHaveCount(1);
  await appCopy(page);
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy requested (table)", {
    timeout: 10000,
  });
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>)["__TABLE_COPY_LAST__"],
        ),
      { timeout: 10000 },
    )
    .toMatchObject({ kind: "cell", r: 0, c: 0 });

  // Bus copy-right reuses the row handler (no duplicate path).
  await busCopy(page, "copy-right", "table");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>)["__TABLE_COPY_LAST__"],
        ),
      { timeout: 10000 },
    )
    .toMatchObject({ kind: "row", r: 0 });

  // Foreign-kind dispatch never leaks across views.
  const before = await page.evaluate(
    () => JSON.stringify((window as unknown as Record<string, unknown>)["__TABLE_COPY_LAST__"]),
  );
  await busCopy(page, "copy", "folder");
  await page.waitForTimeout(300);
  const after = await page.evaluate(
    () => JSON.stringify((window as unknown as Record<string, unknown>)["__TABLE_COPY_LAST__"]),
  );
  expect(after).toBe(before);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3B folder: App copy forwards on bus (selected copyPath), foreign kind isolated", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "folder", "/tmp/l3b-folder-left", "/tmp/l3b-folder-right", "view-folder");
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 15000 })
    .toBeGreaterThan(0);

  // Select the first file via the bus, then drive the App copy path.
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("autocompare:view-command", { detail: { cmd: "next", viewKind: "folder" } }),
    );
  });
  await expect
    .poll(async () => page.locator('[data-testid^="folder-row-"][data-selected="true"]').count(), {
      timeout: 10000,
    })
    .toBe(1);
  const selected = await page
    .locator('[data-testid^="folder-row-"][data-selected="true"]')
    .first()
    .getAttribute("data-path");

  await appCopy(page);
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy requested (folder)", {
    timeout: 10000,
  });
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>)["__FOLDER_COPY_LAST__"],
        ),
      { timeout: 10000 },
    )
    .toMatchObject({ relPath: selected });

  // Foreign-kind dispatch never leaks across views.
  const before = await page.evaluate(
    () => JSON.stringify((window as unknown as Record<string, unknown>)["__FOLDER_COPY_LAST__"]),
  );
  await busCopy(page, "copy", "hex");
  await page.waitForTimeout(300);
  const after = await page.evaluate(
    () => JSON.stringify((window as unknown as Record<string, unknown>)["__FOLDER_COPY_LAST__"]),
  );
  expect(after).toBe(before);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3B hex: App copy forwards on bus (hex/ascii), foreign kind isolated", async ({ page }) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "hex", "/tmp/l3b-hex-left.bin", "/tmp/l3b-hex-right.bin", "view-hex");
  const status = page.getByTestId("hex-copy-status");
  await expect(status).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("hexview/byte-0")).toBeVisible({ timeout: 15000 });

  // Select byte offset 1, then read its honest values from the footer
  // detail (mock serves a real 64-byte fixture here, not the embedded
  // 11-byte fallback — so derive, never hardcode).
  const byte1 = page.getByTestId("hexview/byte-1");
  await byte1.click();
  await expect(byte1).toHaveAttribute("data-selected", "true");
  const detailLeft = (await page.getByTestId("hex-detail-left").textContent()) ?? "";
  const hexVal = detailLeft.match(/left ([0-9a-f]{2})/)?.[1] ?? "";
  expect(hexVal).not.toBe("");
  const asciiChar = detailLeft.match(/'(.?)'/)?.[1] ?? "";
  expect(asciiChar).not.toBe("");

  // App copy path: status + hex receipt for the selection.
  await appCopy(page);
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy requested (hex)", {
    timeout: 10000,
  });
  await expect(status).toContainText("hex", { timeout: 10000 });
  await expect(status).toContainText(hexVal);

  // Bus copy-right reuses the ascii handler for the same selection.
  await busCopy(page, "copy-right", "hex");
  await expect(status).toContainText("ascii", { timeout: 10000 });
  await expect(status).toContainText(asciiChar);

  // Foreign-kind dispatch never leaks into hex.
  const before = (await status.textContent()) ?? "";
  await busCopy(page, "copy", "sync");
  await page.waitForTimeout(300);
  expect(await status.textContent()).toBe(before);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3B sync: App copy forwards on bus, view parks honestly (plan untouched)", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "sync", "/tmp/l3b-sync-left", "/tmp/l3b-sync-right", "view-sync");
  await expect
    .poll(async () => page.getByTestId("syncview/row").count(), { timeout: 15000 })
    .toBeGreaterThan(0);
  const rowsBefore = await page.getByTestId("syncview/row").count();

  // App copy path forwards; the view parks (no copy affordance on plan rows).
  await appCopy(page);
  await expect(page.getByTestId("statusbar/message")).toContainText("Copy requested (sync)", {
    timeout: 10000,
  });
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>)["__SYNC_COPY_PARKED__"],
        ),
      { timeout: 10000 },
    ).toBe(1);
  expect(await page.getByTestId("syncview/row").count()).toBe(rowsBefore);

  // Bus copy-right parks the same way (counter proves the dispatch landed).
  await busCopy(page, "copy-right", "sync");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => (window as unknown as Record<string, unknown>)["__SYNC_COPY_PARKED__"],
        ),
      { timeout: 10000 },
    ).toBe(2);

  // Foreign-kind dispatch never leaks into sync.
  await busCopy(page, "copy", "hex");
  await page.waitForTimeout(300);
  expect(
    await page.evaluate(
      () => (window as unknown as Record<string, unknown>)["__SYNC_COPY_PARKED__"],
    ),
  ).toBe(2);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});

test("L3B media: App copy is an honest park, never a crash", async ({ page }) => {
  const { consoleErrors, pageErrors } = trackErrors(page);
  await createSession(page, "media", "/tmp/l3b-m-left.png", "/tmp/l3b-m-right.png", "view-media");
  await expect(page.getByTestId("mediaview/summary")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId("mediaview/score")).toContainText(/bytes differ/);

  // Media is a summary view (no copy affordance): App parks, no dispatch.
  await appCopy(page);
  await expect(page.getByTestId("statusbar/message")).toContainText(
    'Copy is not available for kind "media"',
    { timeout: 10000 },
  );
  await page.waitForTimeout(300);
  await expect(page.getByTestId("mediaview/summary")).toBeVisible();
  await expect(page.getByTestId("mediaview/score")).toContainText(/bytes differ/);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
