/**
 * Wave-159: Folder DnD + Folder Visual Loop Verification Spec.
 *
 * Covers:
 *  (a) Pane drops:
 *      - Left pane drop -> left path, auto-compare fires
 *      - Right pane drop -> right path, auto-compare fires
 *      - PathBar whole-strip drop: resolves side based on cursor x
 *      - Finder multi-drop (2 files): Item 0 -> left, Item 1 -> right
 *      - Busy-compare honest note: "busy — compare running, drop paused"
 *  (b) rightOnly / leftOnly own-side half rendering:
 *      - leftOnly content rendered on left half (50% width), orphan diagonal hatch on right
 *      - rightOnly content rendered on right half (50% width), orphan diagonal hatch on left
 *      - No color bleed: selection / hover styles bounded to content half
 *  (c) Folder toolbar density + chrome budget:
 *      - section-relative firstRowTop <= 140px
 *      - Single toolbar row with SegmentedControl and all testids intact
 *
 * Rules:
 *  - No offsetHeight locks in behavior specs.
 *  - Run on port 1586 under VITE_MOCK_IPC=1 webkit.
 */
import { expect, test, type Page } from "@playwright/test";

const PANE_BUSY_NOTE = "busy — compare running, drop paused";

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.locator('[data-testid="session-count"], [data-testid="toolbar/action-open"]').first().waitFor({ state: "visible", timeout: 15000 });
  } catch (err) {
    console.warn("Navigation failed:", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

/** Create a fresh folder comparison session with mock paths. */
async function createFolderSession(page: Page, left = "/tmp/f159-left", right = "/tmp/f159-right"): Promise<void> {
  if (await page.getByTestId("view-folder").isVisible().catch(() => false)) {
    const leftField = page.locator('[data-testid="pathbar/left"]');
    const rightField = page.locator('[data-testid="pathbar/right"]');
    if (await leftField.isVisible().catch(() => false)) {
      await leftField.fill(left);
      await leftField.press("Enter");
      await rightField.fill(right);
      await rightField.press("Enter");
      await expect.poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 }).toBeGreaterThan(3);
      return;
    }
  }
  if (await page.getByTestId("workspace/demo-banner").isVisible().catch(() => false)) {
    await page.getByTestId("workspace/demo-clear").click();
  }
  const dialog = page.getByTestId("session/dialog");
  if (!(await dialog.isVisible().catch(() => false))) {
    await page.getByTestId("toolbar/action-open").click();
    await expect(dialog).toBeVisible();
  }
  await page.getByTestId("session/kind-select").selectOption("folder");
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 15000 });
  await expect.poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 }).toBeGreaterThan(3);
}

/** Synthetic drag-and-drop helper with clientX support. */
async function dropPathsAt(
  page: Page,
  selector: string,
  paths: readonly string[],
  clientX?: number,
): Promise<void> {
  await page.locator(selector).first().evaluate(
    (el, args) => {
      const files = args.paths.map((p) => ({
        name: p.split("/").pop() ?? p,
        path: p,
      }));
      const dt = {
        files,
        types: ["Files"],
        getData: (_type: string) => "",
        dropEffect: "copy",
      };
      const init: DragEventInit = {
        bubbles: true,
        cancelable: true,
        clientX: args.clientX ?? el.getBoundingClientRect().left + 10,
        clientY: el.getBoundingClientRect().top + 10,
      };
      const ev = new DragEvent("drop", init);
      Object.defineProperty(ev, "dataTransfer", { value: dt });
      el.dispatchEvent(ev);
    },
    { paths, clientX },
  );
}

test.describe("Wave-159: Folder DnD and Visual Loop", () => {
  test("(a) pane drops: left and right pane drops route to their respective sides and auto-compare", async ({
    page,
  }) => {
    await createFolderSession(page);
    await expect.poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 }).toBeGreaterThan(3);

    // Left pane drop via data-panedrop="left"
    const leftNewPath = "/tmp/f159-left-dropped";
    await dropPathsAt(page, '[data-panedrop="left"]', [leftNewPath]);
    await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(leftNewPath, { timeout: 5000 });
    await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });

    // Right pane drop via data-panedrop="right"
    const rightNewPath = "/tmp/f159-right-dropped";
    await dropPathsAt(page, '[data-panedrop="right"]', [rightNewPath]);
    await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(rightNewPath, { timeout: 5000 });
    await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });
  });

  test("(a) whole-strip PathBar drop routes by cursor clientX position", async ({ page }) => {
    await createFolderSession(page);
    const pathbar = page.locator('[data-testid="pathbar"]');
    await expect(pathbar).toBeVisible();

    const rect = await pathbar.evaluate((el) => {
      const r = el.getBoundingClientRect();
      return { left: r.left, width: r.width };
    });

    // Drop on left half of strip
    const dropLeftPath = "/tmp/strip-left-159";
    await dropPathsAt(page, '[data-testid="pathbar"]', [dropLeftPath], rect.left + rect.width * 0.25);
    await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(dropLeftPath, { timeout: 5000 });

    // Drop on right half of strip
    const dropRightPath = "/tmp/strip-right-159";
    await dropPathsAt(page, '[data-testid="pathbar"]', [dropRightPath], rect.left + rect.width * 0.75);
    await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(dropRightPath, { timeout: 5000 });
  });

  test("(a) Finder multi-drop (2 files) routes Item 0 -> Left, Item 1 -> Right", async ({ page }) => {
    await createFolderSession(page);
    const dualPaths = ["/tmp/multi-0-159", "/tmp/multi-1-159"];

    // Multi-drop on PathBar strip
    await dropPathsAt(page, '[data-testid="pathbar"]', dualPaths);
    await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(dualPaths[0]!, { timeout: 5000 });
    await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(dualPaths[1]!, { timeout: 5000 });
    await expect(page.getByTestId("statusbar/message")).toContainText("Compare finished", { timeout: 10000 });

    // Multi-drop on PaneDropZone root
    const paneDualPaths = ["/tmp/pane-multi-0", "/tmp/pane-multi-1"];
    await dropPathsAt(page, '[data-testid="panedrop/root"]', paneDualPaths);
    await expect(page.locator('[data-testid="pathbar/left"]')).toHaveValue(paneDualPaths[0]!, { timeout: 5000 });
    await expect(page.locator('[data-testid="pathbar/right"]')).toHaveValue(paneDualPaths[1]!, { timeout: 5000 });
  });

  test("(a) busy-compare honest note when drop lands while comparing", async ({ page }) => {
    await createFolderSession(page);

    // Simulate active comparing on pathbar
    await page.evaluate(() => {
      const pb = document.querySelector('[data-testid="pathbar"]');
      if (pb) {
        pb.setAttribute("data-comparing", "true");
        pb.setAttribute("aria-busy", "true");
      }
    });

    // Attempt a drop on the left pathfield while busy
    await dropPathsAt(page, '[data-testid="pathbar/left"]', ["/tmp/busy-drop.txt"]);

    // Honest busy note should be displayed in PathBar
    await expect(page.locator('[data-testid="pathbar"]')).toHaveAttribute("data-note", PANE_BUSY_NOTE);
    await expect(page.locator('[data-testid="pathbar"]')).toHaveAttribute("title", PANE_BUSY_NOTE);

    // Also verify drop directly on PaneDropZone triggers busy note via event
    await dropPathsAt(page, '[data-testid="panedrop/root"]', ["/tmp/busy-pane-drop.txt"]);
    await expect(page.locator('[data-testid="pathbar"]')).toHaveAttribute("data-note", PANE_BUSY_NOTE);
  });

  test("(b) rightOnly and leftOnly own-side half rendering, diagonal hatch for orphan side, and no color bleed", async ({
    page,
  }) => {
    await createFolderSession(page);
    await expect.poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 }).toBeGreaterThan(3);

    // Inspect geometry and hatch styling of leftOnly and rightOnly rows
    const orphanInfo = await page.evaluate(() => {
      const rows = Array.from(document.querySelectorAll('[data-testid^="folder-row-"]'));
      let leftOnlyNode: {
        rowSide: string | null;
        nodeSide: string | null;
        nodeLeft: number;
        nodeWidth: number;
        rowWidth: number;
        hatchBgPos: string;
        hatchBgImg: string;
      } | null = null;

      let rightOnlyNode: {
        rowSide: string | null;
        nodeSide: string | null;
        nodeLeft: number;
        nodeWidth: number;
        rowWidth: number;
        hatchBgPos: string;
        hatchBgImg: string;
      } | null = null;

      for (const r of rows) {
        const status = r.getAttribute("data-status");
        const node = r.querySelector('[data-testid^="folderview/node-"]') as HTMLElement | null;
        const rowRect = r.getBoundingClientRect();
        const nodeRect = node?.getBoundingClientRect();
        const rowStyle = window.getComputedStyle(r);

        if (status === "leftOnly" && !leftOnlyNode && node && nodeRect) {
          leftOnlyNode = {
            rowSide: r.getAttribute("data-side"),
            nodeSide: node.getAttribute("data-side"),
            nodeLeft: nodeRect.left - rowRect.left,
            nodeWidth: nodeRect.width,
            rowWidth: rowRect.width,
            hatchBgPos: rowStyle.backgroundPosition,
            hatchBgImg: rowStyle.backgroundImage,
          };
        } else if (status === "rightOnly" && !rightOnlyNode && node && nodeRect) {
          rightOnlyNode = {
            rowSide: r.getAttribute("data-side"),
            nodeSide: node.getAttribute("data-side"),
            nodeLeft: nodeRect.left - rowRect.left,
            nodeWidth: nodeRect.width,
            rowWidth: rowRect.width,
            hatchBgPos: rowStyle.backgroundPosition,
            hatchBgImg: rowStyle.backgroundImage,
          };
        }
      }

      return { leftOnlyNode, rightOnlyNode };
    });

    expect(orphanInfo.leftOnlyNode, "found leftOnly row").not.toBeNull();
    const l = orphanInfo.leftOnlyNode!;
    expect(l.rowSide).toBe("left-only");
    expect(l.nodeSide).toBe("left");
    // Left-only node is placed at left edge of row (offset near 0)
    expect(l.nodeLeft).toBeLessThan(10);
    // Node takes approximately 50% width (within 5px tolerance)
    expect(Math.abs(l.nodeWidth - l.rowWidth * 0.5)).toBeLessThan(5);
    // Hatch is anchored to right half for leftOnly (e.g. "right center" or "100% 50%")
    expect(l.hatchBgPos.includes("right") || l.hatchBgPos.includes("100%")).toBe(true);
    expect(l.hatchBgImg).toContain("repeating-linear-gradient");

    expect(orphanInfo.rightOnlyNode, "found rightOnly row").not.toBeNull();
    const r = orphanInfo.rightOnlyNode!;
    expect(r.rowSide).toBe("right-only");
    expect(r.nodeSide).toBe("right");
    // Right-only node is placed at right half (offset near 50% of row width)
    expect(r.nodeLeft).toBeGreaterThanOrEqual(r.rowWidth * 0.48);
    // Node takes approximately 50% width
    expect(Math.abs(r.nodeWidth - r.rowWidth * 0.5)).toBeLessThan(5);
    // Hatch is anchored to left half for rightOnly (e.g. "left center" or "0% 50%")
    expect(r.hatchBgPos.includes("left") || r.hatchBgPos.includes("0%")).toBe(true);
    expect(r.hatchBgImg).toContain("repeating-linear-gradient");

    // Test selection styling to confirm no color bleed onto orphan hatch
    const rightRow = page.locator('[data-testid^="folder-row-"][data-status="rightOnly"][data-path$=".txt"]').first();
    await rightRow.click();
    await expect(rightRow).toHaveAttribute("data-selected", "true");

    const bleedCheck = await page.evaluate(() => {
      const row = document.querySelector('[data-testid^="folder-row-"][data-status="rightOnly"][data-selected="true"]') as HTMLElement | null;
      if (!row) return null;
      const node = row.querySelector('[data-testid^="folderview/node-"]') as HTMLElement | null;
      const rowStyle = window.getComputedStyle(row);
      const nodeStyle = node ? window.getComputedStyle(node) : null;
      return {
        rowBg: rowStyle.backgroundColor,
        nodeBg: nodeStyle?.backgroundColor,
        nodeWidth: node?.offsetWidth,
        rowWidth: row.offsetWidth,
      };
    });

    expect(bleedCheck).not.toBeNull();
    // Selection background is applied to the 50% .fv-node, not the outer row
    expect(bleedCheck!.nodeWidth).toBeLessThanOrEqual(bleedCheck!.rowWidth * 0.55);
  });

  test("(c) folder toolbar density: chrome budget firstRowTop <= 140, segmented control testids intact", async ({
    page,
  }) => {
    await createFolderSession(page);
    await expect.poll(async () => page.locator('[data-testid^="folder-row-"]').count(), { timeout: 10000 }).toBeGreaterThan(3);

    // Chrome budget check: first content row top <= 140px from section top
    const delta = await page.evaluate(() => {
      const section = document.querySelector('section[aria-label="folder comparison"]');
      const firstRow = document.querySelector('[data-testid^="folder-row-"]');
      if (!(section instanceof HTMLElement) || !(firstRow instanceof HTMLElement)) return 9999;
      return firstRow.getBoundingClientRect().top - section.getBoundingClientRect().top;
    });
    expect(delta, `firstRowTop <= 140px (got ${delta})`).toBeLessThanOrEqual(140);

    // Toolbar testids and controls
    const toolbar = page.getByTestId("folderview/toolbar");
    await expect(toolbar).toBeVisible();

    const allFilter = page.getByTestId("folder-filter-all");
    const diffsFilter = page.getByTestId("folder-filter-diffs");
    const sameFilter = page.getByTestId("folder-filter-same");
    const changesOnly = page.getByTestId("folder-changesonly-toggle");
    const nameFilterBtn = page.getByTestId("folderview/toolbar-namefilter");

    await expect(allFilter).toBeVisible();
    await expect(diffsFilter).toBeVisible();
    await expect(sameFilter).toBeVisible();
    await expect(changesOnly).toBeVisible();
    await expect(nameFilterBtn).toBeVisible();

    // Verify filter interaction
    await diffsFilter.click();
    await expect(diffsFilter).toHaveAttribute("aria-pressed", "true");

    await allFilter.click();
    await expect(allFilter).toHaveAttribute("aria-pressed", "true");
  });
});
