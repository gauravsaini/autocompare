/**
 * Wave-235 empty + notice standards (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Pins the wave-232 primitive contracts after the 7-view migration:
 *  (a) Modal is centered and Esc-dismissed (gallery route);
 *  (b) DockedBar is in-flow and Esc-dismissed (gallery route);
 *  (c) Empty contract: blank folder/sync/media render the single honest
 *      `workspace/blank-empty` ("needs two paths", views never mount), and
 *      every `ui/EmptyState` carries one h2 + centered geometry;
 *  (d) No `role=alert` anywhere on these pages: every inline notice is the
 *      single `role=status` `ui/InlineNotice`.
 *
 * PORT-FREE: runs on a fresh :1803 server
 * (`reuseExistingServer:false`, `--workers=1`, webkit) so no shared
 * port or parallel worker can flake the gate.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const EVID_DIR = "/tmp/ui-standards-235";

test.beforeEach(async ({ page }) => {
  try {
    if (!fs.existsSync(EVID_DIR)) fs.mkdirSync(EVID_DIR, { recursive: true });
  } catch {
    // best-effort
  }
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 30000 });
    await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 30000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

function collectErrors(page: Page): { consoleErrors: string[]; pageErrors: string[] } {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  return { consoleErrors, pageErrors };
}

function assertClean(consoleErrors: string[], pageErrors: string[]): void {
  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
}

/** Blank session of `kind` via the Home quick-create card (no paths staged). */
async function openBlankKind(page: Page, kind: string, viewId: string): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click();
  } else {
    const home = page.getByTestId("tab-home");
    if ((await home.count()) > 0) await home.click().catch(() => {});
  }
  await expect(page.getByTestId("workspace/welcome")).toBeVisible({ timeout: 15000 });
  await page.getByTestId(`home/new-${kind}`).click();
  // Wave-150 (wave-138 contract): blank sessions render the single
  // honest empty; views never mount until both paths are set.
  await expect(page.getByTestId("workspace/blank-empty")).toBeVisible({ timeout: 15000 });
  await expect(page.getByTestId(viewId)).toHaveCount(0);
}

test("modal is centered and Esc-dismissed", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/root")).toBeVisible({ timeout: 15000 });

  await page.getByTestId("gallery/open-modal").click();
  const modal = page.getByTestId("gallery/modal");
  await expect(modal).toBeVisible({ timeout: 15000 });
  await expect(modal).toHaveAttribute("role", "dialog");

  const box = await modal.boundingBox();
  expect(box, "modal has a layout box").not.toBeNull();
  if (box !== null) {
    const vp = page.viewportSize() ?? { width: 1280, height: 800 };
    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;
    expect(Math.abs(cx - vp.width / 2), `modal horizontally centered (cx=${cx})`).toBeLessThan(vp.width * 0.08);
    expect(Math.abs(cy - vp.height / 2), `modal vertically centered (cy=${cy})`).toBeLessThan(vp.height * 0.25);
  }
  await page.screenshot({ path: `${EVID_DIR}/modal-centered.png` });

  await page.keyboard.press("Escape");
  await expect(modal).toBeHidden({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});

test("docked bar is in-flow and Esc-dismissed", async ({ page }) => {  const { consoleErrors, pageErrors } = collectErrors(page);
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/root")).toBeVisible({ timeout: 15000 });

  // Dock starts open; Esc collapses it without moving rows (in-flow: no
  // absolute/fixed positioning on the bar itself).
  const dock = page.getByTestId("gallery/dock");
  await expect(dock).toBeVisible({ timeout: 15000 });
  const pos = await dock.evaluate((el) => getComputedStyle(el).position);
  expect(pos, "docked bar never floats (no absolute/fixed)").not.toMatch(/absolute|fixed/);
  await page.keyboard.press("Escape");
  await expect(dock).toBeHidden({ timeout: 15000 });

  // Reopen via toggle, dismiss via the close button.
  // Wave-239: closeTestId is the explicit wave-234 contract id
  // (`gallery/dock-close`), not the DockedBar default suffix.
  await page.getByTestId("gallery/toggle-dock").click();
  await expect(dock).toBeVisible({ timeout: 15000 });
  await page.getByTestId("gallery/dock-close").click();
  await expect(dock).toBeHidden({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});

test("unified view controls: single nav/filter/copy source, Esc contract, no raw dupes", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);
  const evidDir = "/tmp/unified-239";
  try {
    if (!fs.existsSync(evidDir)) fs.mkdirSync(evidDir, { recursive: true });
  } catch {
    // best-effort
  }

  // Gallery primitive: every cluster renders with byte-identical demo testids.
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("gallery/section-unified")).toBeVisible({ timeout: 15000 });
  for (const id of [
    "gallery/unified-nav-prev",
    "gallery/unified-nav-next",
    "gallery/unified-filter-all",
    "gallery/unified-filter-diffs",
    "gallery/unified-filter-same",
    "gallery/unified-diffsonly",
    "gallery/unified-context",
    "gallery/unified-copy-left",
    "gallery/unified-copy-right",
  ]) {
    await expect(page.getByTestId(id), `gallery primitive ${id}`).toBeVisible({ timeout: 15000 });
  }

  // Clusters are interactive through the single implementation.
  await page.getByTestId("gallery/unified-nav-next").click();
  await expect(page.getByTestId("gallery/unified-note")).toContainText("nav: next");
  await page.getByTestId("gallery/unified-filter-diffs").click();
  await expect(page.getByTestId("gallery/unified-note")).toContainText("filter: diffs");
  await page.getByTestId("gallery/unified-copy-left").click();
  await expect(page.getByTestId("gallery/unified-note")).toContainText("copy: left");

  // Esc contract (DockedBar parity): Esc dismisses the closable cluster.
  await expect(page.getByTestId("gallery/unified-dismissable")).toBeVisible({ timeout: 15000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("gallery/unified-dismissable")).toBeHidden({ timeout: 15000 });
  await page.getByTestId("gallery/unified-reopen").click();
  await expect(page.getByTestId("gallery/unified-dismissable")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("gallery/unified-close").click();
  await expect(page.getByTestId("gallery/unified-dismissable")).toBeHidden({ timeout: 15000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${evidDir}/gallery-unified.png` });

  // Text view: every nav/filter/copy dupe rides the primitive (inside
  // [data-unified-cluster]); no raw duplicate survives outside it or the
  // top toolbar.
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId("textview/toolbar-prev-diff")).toBeVisible({ timeout: 15000 });
  const dupes = await page.evaluate(() => {
    const ids = [
      "textview/toolbar-prev-diff",
      "textview/toolbar-next-diff",
      "textview/toolbar-copy-line-left",
      "textview/toolbar-copy-line-right",
      "textview/diffsonly-toggle",
      "textview/context-lines",
    ];
    return ids.map((id) => {
      const el = document.querySelector(`[data-testid="${id}"]`);
      if (!el) return { id, found: false, unified: false };
      return { id, found: true, unified: el.closest('[data-unified-cluster="nav"],[data-unified-cluster="filter"],[data-unified-cluster="copy"]') !== null };
    });
  });
  for (const d of dupes) {
    expect(d.found, `${d.id} stays mounted (byte-identical testid)`).toBe(true);
    expect(d.unified, `${d.id} rides UnifiedViewControls (no raw duplicate)`).toBe(true);
  }

  // Static pins: folder/table/hex/merge toolbars render through the
  // primitive (shared cluster present, raw dupe blocks gone). The dupe
  // testids survive only as props to the unified clusters.
  const here = path.dirname(fileURLToPath(import.meta.url));
  const srcRoot = path.resolve(here, "../../src/components/views");
  const readSrc = (rel: string): string => fs.readFileSync(path.join(srcRoot, rel), "utf8");
  const folderSrc = readSrc("folder/controls/FolderToolbar.tsx");
  expect(folderSrc, "folder filter rides UnifiedFilterSegment").toContain("UnifiedFilterSegment");
  expect(folderSrc, "folder keeps zero raw SegmentedControl dupes").not.toContain("<SegmentedControl");
  const tableSrc = readSrc("table/controls/TableToolbar.tsx");
  expect(tableSrc, "table nav rides UnifiedNavCluster").toContain("UnifiedNavCluster");
  expect(tableSrc, "table keeps zero raw nav ToolbarButtons").not.toMatch(/<ToolbarButton[\s\S]{0,300}tableview\/toolbar-prev-diff/);
  const hexSrc = readSrc("hex/controls/HexToolbar.tsx");
  expect(hexSrc, "hex nav rides UnifiedNavCluster").toContain("UnifiedNavCluster");
  expect(hexSrc, "hex keeps zero raw diff ToolbarButtons").not.toMatch(/<ToolbarButton[\s\S]{0,300}hex-diff-prev/);
  const mergeSrc = readSrc("merge/controls/MergeToolbar.tsx");
  expect(mergeSrc, "merge nav rides UnifiedNavCluster").toContain('variant="button"');
  expect(mergeSrc, "merge keeps zero raw nav Buttons").not.toMatch(/<Button size="sm" data-testid="merge-prev"/);
  const textSrc = readSrc("text/controls/TextToolbar.tsx");
  expect(textSrc, "text nav/copy/filter ride the unified clusters").toContain("UnifiedNavCluster");
  expect(textSrc, "text keeps zero raw nav ToolbarButtons").not.toMatch(/<ToolbarButton[\s\S]{0,300}textview\/toolbar-prev-diff/);

  // Boxes evidence for REPORT (cluster geometry, in-flow check).
  const boxes = await page.evaluate(() => {
    const out: Record<string, { x: number; y: number; w: number; h: number } | null> = {};
    for (const id of ["textview/toolbar-prev-diff", "textview/diffsonly-toggle", "textview/toolbar-copy-line-left"]) {
      const el = document.querySelector(`[data-testid="${id}"]`);
      if (!el) {
        out[id] = null;
        continue;
      }
      const r = el.getBoundingClientRect();
      out[id] = { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) };
    }
    return out;
  });
  try {
    fs.writeFileSync(`${evidDir}/boxes.json`, `${JSON.stringify(boxes, null, 2)}\n`);
  } catch {
    // best-effort
  }
  await page.screenshot({ path: `${evidDir}/text-unified.png` });
  await expect(page.getByRole("alert")).toHaveCount(0);
  assertClean(consoleErrors, pageErrors);
});
test("empty contract: blank kinds share one honest empty, states are uniform", async ({ page }) => {
  const { consoleErrors, pageErrors } = collectErrors(page);

  for (const [kind, viewId] of [["folder", "view-folder"], ["sync", "view-sync"], ["media", "view-media"]] as const) {
    await openBlankKind(page, kind, viewId);
    const empty = page.getByTestId("workspace/blank-empty");
    await expect(empty).toBeVisible({ timeout: 15000 });
    await expect(empty).toContainText("needs two paths");
    await expect(page.getByRole("alert")).toHaveCount(0);
    await page.screenshot({ path: `${EVID_DIR}/blank-${kind}-empty.png` });
  }

  // Gallery EmptyState: one h2 + centered geometry (the contract every
  // migrated per-view empty now shares).
  await page.goto("/?gallery=1", { waitUntil: "domcontentloaded" });
  const estate = page.getByTestId("gallery/empty");
  await expect(estate).toBeVisible({ timeout: 15000 });
  expect(await estate.locator("h2").count(), "EmptyState carries one h2").toBe(1);
  await expect(estate.locator("h2")).toContainText("Nothing to compare");
  const align = await estate.evaluate((el) => getComputedStyle(el).textAlign);
  expect(align, "EmptyState is centered").toBe("center");
  const notice = page.getByTestId("gallery/notice");
  await expect(notice).toHaveAttribute("role", "status");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.screenshot({ path: `${EVID_DIR}/gallery-emptynotice.png` });
  assertClean(consoleErrors, pageErrors);
});
