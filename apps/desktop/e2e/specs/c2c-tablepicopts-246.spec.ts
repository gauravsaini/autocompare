/**
 * Wave-C2c table+picture options proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Exposes the engine-ready options end to end WITHOUT touching disk:
 *  - table `delimiter` select (`tableview/delimiter-select`) + numeric
 *    `epsilon` input (`tableview/numeric-tolerance`) ride the single
 *    `TableView` toolbar row and re-fire `table_compare` on change
 *    (mock echoes the same fixture, so the grid reloads at
 *    `100 / 550 rows` with zero errors);
 *  - the typed `tableCompare(..., opts)` wrapper sends `delimiter` +
 *    all three tolerance spellings (`numericTolerance` /
 *    `numeric_tolerance` / `epsilon`) and records the last request on
 *    `window.__TABLE_PICOPTS_LAST__`; the legacy 4-field wire
 *    (no `opts`) records nulls and stays byte-identical;
 *  - the typed `pictureCompare(left, right, tolerance)` wrapper sends
 *    the new optional tolerance and records it on
 *    `window.__PICTURE_TOLERANCE_LAST__` (legacy 2-arg call records
 *    null); real honoring is covered by Rust unit tests in
 *    `commands/media.rs`;
 *  - the media tolerance slider (`mediaview/tolerance-slider`) is
 *    wired to the same tolerance state + re-compare path in audio
 *    mode (arrow keys move it, the text box follows, Recompare
 *    reloads the summary with zero errors).
 *
 * Mock honesty: the mock adapter has no delimiter/tolerance engine
 * (fixed fixtures), so toggles are verified at the IPC-param path
 * (explicit params on the wire + grid reload), never a crash. Picture
 * mode keeps its mode-only chrome per mediachrome-72 (no tolerance
 * toggle there), so picture tolerance is proven at the typed-wrapper
 * path. Skip policy matches the other specs (infra failures skip).
 * No geometry asserts, no screenshots (never in-repo). New file only;
 * does not touch any existing spec.
 */
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

async function createTableSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible();
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0");

  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption("table");
  await page.getByTestId("session/left-path").fill("/tmp/wave246-left.csv");
  await page.getByTestId("session/right-path").fill("/tmp/wave246-right.csv");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId("view-table")).toBeVisible();
}

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

async function picoptsLast(page: Page): Promise<{ delimiter: unknown; numericTolerance: unknown }> {
  return (await page.evaluate(
    () => (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"],
  )) as { delimiter: unknown; numericTolerance: unknown };
}

test("c2c table+picture options: delimiter + epsilon + tolerance controls re-compare", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // ---- Table: typed wrapper legacy vs extended. ----
  const legacy = await page.evaluate(async () => {
    const mod = await import("/src/ipc/table.ts");
    const res = (await mod.tableCompare("left-a", "right-b", false)) as unknown as Record<
      string,
      unknown
    >;
    const last = (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"];
    return { res, last };
  });
  expect((legacy.res as Record<string, unknown>)["sessionId"]).toBe("mock-table-1");
  expect(legacy.last as Record<string, unknown>).toEqual({
    delimiter: null,
    numericTolerance: null,
  });

  const extended = await page.evaluate(async () => {
    const mod = await import("/src/ipc/table.ts");
    const res = (await mod.tableCompare("left-a", "right-b", false, [], {
      delimiter: ";",
      numericTolerance: 0.01,
    })) as unknown as Record<string, unknown>;
    const last = (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"];
    return { res, last };
  });
  expect((extended.res as Record<string, unknown>)["sessionId"]).toBe("mock-table-1");
  expect(extended.last as Record<string, unknown>).toEqual({
    delimiter: ";",
    numericTolerance: 0.01,
  });

  // `epsilon` spelling wins identically (first-Some on the Rust side).
  const viaEpsilon = await page.evaluate(async () => {
    const mod = await import("/src/ipc/table.ts");
    await mod.tableCompare("left-a", "right-b", false, [], { epsilon: 0.5 });
    return (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"];
  });
  expect(viaEpsilon as Record<string, unknown>).toEqual({
    delimiter: null,
    numericTolerance: 0.5,
  });

  // ---- Table: UI controls re-fire the compare. ----
  await createTableSession(page);
  const view = page.getByTestId("view-table");
  await expect(view).toContainText("100 / 550 rows");

  const delimiterSelect = page.getByTestId("tableview/delimiter-select");
  const epsilonInput = page.getByTestId("tableview/numeric-tolerance");
  await expect(delimiterSelect).toBeVisible();
  await expect(delimiterSelect).toHaveValue(",");
  await expect(epsilonInput).toBeVisible();
  await expect(epsilonInput).toHaveValue("");

  await delimiterSelect.selectOption(";");
  await expect
    .poll(async () => (await picoptsLast(page)).delimiter, { timeout: 10000 })
    .toBe(";");
  await expect(view).toContainText("100 / 550 rows");

  await epsilonInput.fill("0.01");
  await expect
    .poll(async () => (await picoptsLast(page)).numericTolerance, { timeout: 10000 })
    .toBe(0.01);
  await expect(view).toContainText("100 / 550 rows");
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Back to defaults: hook records nulls again (legacy-shaped request).
  await delimiterSelect.selectOption(",");
  await epsilonInput.fill("");
  await expect.poll(async () => await picoptsLast(page), { timeout: 10000 }).toEqual({
    delimiter: null,
    numericTolerance: null,
  });
  await expect(view).toContainText("100 / 550 rows");

  // ---- Picture: typed wrapper tolerance path. ----
  const picLegacy = await page.evaluate(async () => {
    const mod = await import("/src/ipc/media.ts");
    const res = (await mod.pictureCompare("left-p", "right-p")) as unknown as Record<
      string,
      unknown
    >;
    const last = (window as unknown as Record<string, unknown>)["__PICTURE_TOLERANCE_LAST__"];
    return { res, last };
  });
  expect((picLegacy.res as Record<string, unknown>)["receipt"]).toBe("mock-picture-1");
  expect(picLegacy.last).toBe(null);

  const picTol = await page.evaluate(async () => {
    const mod = await import("/src/ipc/media.ts");
    const res = (await mod.pictureCompare("left-p", "right-p", 16)) as unknown as Record<
      string,
      unknown
    >;
    const last = (window as unknown as Record<string, unknown>)["__PICTURE_TOLERANCE_LAST__"];
    return { res, last };
  });
  expect((picTol.res as Record<string, unknown>)["receipt"]).toBe("mock-picture-1");
  expect(picTol.last).toBe(16);

  // ---- Audio: slider drives the same tolerance state + re-compare. ----
  await createMediaSession(page, "/tmp/m-left.wav", "/tmp/m-right.wav");
  await expect(page.getByTestId("mediaview/mode")).toContainText(/Audio compare/);
  await expect(page.getByTestId("mediaview/score")).toContainText(/3 \/ 100 windows differ/);
  const slider = page.getByTestId("mediaview/tolerance-slider");
  const tolBox = page.getByTestId("mediaview/tolerance");
  await expect(slider).toBeVisible();
  await expect(tolBox).toBeVisible();
  // NOTE: no `slider.click()` — clicking a range track natively jumps
  // to the click position. `focus()` + arrows moves from the current
  // value deterministically.
  await slider.focus();
  await slider.press("ArrowRight");
  await slider.press("ArrowRight");
  await slider.press("ArrowRight");
  await expect(tolBox).toHaveValue("3");
  // The panel is a Modal (backdrop covers the toolbar Recompare while
  // open), so re-compare via the panel's own button — same handler.
  await page
    .getByTestId("mediaview/tolerance-panel")
    .getByRole("button", { name: "Recompare" })
    .click();
  await expect(page.getByTestId("mediaview/score")).toContainText(/windows differ/);
  await expect(page.getByRole("alert")).toHaveCount(0);

  // Typed audio path: the mock echoes the applied tolerance.
  const audTol = await page.evaluate(async () => {
    const mod = await import("/src/ipc/media.ts");
    const res = (await mod.audioCompare("left-a", "right-a", 5)) as unknown as Record<
      string,
      unknown
    >;
    return res["tolerance"];
  });
  expect(audTol).toBe(5);

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
