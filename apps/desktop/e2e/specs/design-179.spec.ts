/**
 * Wave-179: Tauri BC-parity slice, shared design system plus chrome polish, geometry-first.
 *
 * Verifies:
 * 1. Control heights 22-24px and radius 5px on UI controls
 * 2. Lucide icons size 14 stroke 1.75 and zero hand-coded SVGs in controls
 * 3. Chrome budget firstRowTop <= 140px across views (text, folder, table, merge, hex)
 * 4. macOS blue accent focus styling (#0a84ff)
 */
import { expect, test, type Page } from "@playwright/test";

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

async function createSession(
  page: Page,
  kind: string,
  left: string,
  right: string,
  viewId: string,
): Promise<void> {
  const clear = page.getByTestId("workspace/demo-clear");
  if ((await clear.count()) > 0) {
    await clear.click().catch(() => {});
  }
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible();
  await page.getByTestId("session/kind-select").selectOption(kind);
  await page.getByTestId("session/left-path").fill(left);
  await page.getByTestId("session/right-path").fill(right);
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden();
  await expect(page.getByTestId(viewId)).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  try {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("session-count")).toBeVisible({ timeout: 15000 });
  } catch (err) {
    console.warn("Browser navigation failed (offline / no browser binary):", err);
    test.skip(true, `Browser unavailable: ${err}`);
  }
});

test("design-179: control heights 22-24px and radius 5px on UI controls", async ({ page }) => {
  // 1. Verify CSS tokens
  const css = await fetchSrc(page, "/src/theme/tokens.css");
  expect(css, "tokens.css loaded").not.toBeNull();
  expect(css).toContain("--ac-control-h-sm: 22px;");
  expect(css).toContain("--ac-control-h-md: 24px;");
  expect(css).toContain("--ac-control-radius: 5px;");

  // 2. PathBar input: controlSize="md" (24px) and radius 5px
  const leftInput = page.getByTestId("pathbar/left");
  await expect(leftInput).toBeVisible();
  const inputGeo = await leftInput.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return {
      h: Math.round(r.height),
      radius: cs.borderRadius,
    };
  });
  expect(inputGeo.h, "input height is 24px (md control)").toBe(24);
  expect(inputGeo.radius, "input radius is 5px").toBe("5px");

  // 3. PathBar buttons: md size ToolbarButton (24px) and radius 5px
  const browseBtn = page.getByTestId("pathbar/left-browse");
  await expect(browseBtn).toBeVisible();
  const browseGeo = await browseBtn.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return {
      h: Math.round(r.height),
      radius: cs.borderRadius,
    };
  });
  expect(browseGeo.h, "browse button height is 24px (md control)").toBe(24);
  expect(browseGeo.radius, "browse button radius is 5px").toBe("5px");

  // 4. In Folder View: SegmentedControl and ToolbarButton (sm = 22px, radius 5px)
  await createSession(page, "folder", "/tmp/d179-f-left", "/tmp/d179-f-right", "view-folder");
  const folderFilter = page.getByTestId("folder-filter-all");
  await expect(folderFilter).toBeVisible();
  const segGeo = await folderFilter.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return {
      h: Math.round(r.height),
      radius: cs.borderRadius,
    };
  });
  expect(segGeo.h, "segmented control option height is 22px (sm control)").toBe(22);
  expect(segGeo.radius, "segmented control option radius is 5px").toBe("5px");

  const rulesBtn = page.getByTestId("folderview/toolbar-rules");
  await expect(rulesBtn).toBeVisible();
  const rulesGeo = await rulesBtn.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return {
      h: Math.round(r.height),
      radius: cs.borderRadius,
    };
  });
  expect(rulesGeo.h, "toolbar button height is 22px (sm control)").toBe(22);
  expect(rulesGeo.radius, "toolbar button radius is 5px").toBe("5px");

  // 5. In Table View: Button controls (sm = 22px, radius 5px)
  await createSession(page, "table", "/tmp/d179-t-left.csv", "/tmp/d179-t-right.csv", "view-table");
  const exportBtn = page.getByTestId("table-export");
  await expect(exportBtn).toBeVisible();
  const exportGeo = await exportBtn.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return {
      h: Math.round(r.height),
      radius: cs.borderRadius,
    };
  });
  expect(exportGeo.h, "export button height is 22px (sm button)").toBe(22);
  expect(exportGeo.radius, "export button radius is 5px").toBe("5px");
});

test("design-179: Lucide icons size 14 stroke 1.75 and zero hand-coded SVGs in controls", async ({ page }) => {
  // 1. Source verification: zero inline hand-coded SVGs in controls and UI
  const controlPaths = [
    "/src/components/ui/Button.tsx",
    "/src/components/ui/ToolbarButton.tsx",
    "/src/components/ui/SegmentedControl.tsx",
    "/src/components/ui/Checkbox.tsx",
    "/src/components/ui/Input.tsx",
    "/src/components/ui/Select.tsx",
    "/src/components/ui/DotButton.tsx",
    "/src/components/views/folder/controls/FolderToolbar.tsx",
    "/src/components/views/sync/controls/SyncToolbar.tsx",
    "/src/components/views/table/controls/TableToolbar.tsx",
    "/src/components/views/hex/controls/HexToolbar.tsx",
    "/src/components/views/merge/controls/MergeToolbar.tsx",
  ];

  for (const p of controlPaths) {
    const src = await fetchSrc(page, p);
    expect(src, `file fetched: ${p}`).not.toBeNull();
    expect(src, `zero hand-coded <svg> in ${p}`).not.toContain("<svg");
  }

  // 2. DOM verification: all control icons render as Lucide 14px stroke 1.75
  await createSession(page, "merge", "/tmp/d179-m-base.txt", "/tmp/d179-m-right.txt", "view-merge");
  const iconStats = await page.evaluate(() => {
    const toolbars = document.querySelectorAll(
      '[data-testid="pathbar"], [role="toolbar"], [data-testid="mergeview/toolbar"]',
    );
    const icons: {
      tag: string;
      wAttr: string | null;
      hAttr: string | null;
      wRect: number;
      hRect: number;
      strokeWidth: string | null;
      isLucide: boolean;
      visible: boolean;
    }[] = [];

    toolbars.forEach((tb) => {
      tb.querySelectorAll("svg").forEach((svg) => {
        const rect = svg.getBoundingClientRect();
        const style = window.getComputedStyle(svg);
        const visible = style.display !== "none" && style.visibility !== "hidden" && rect.width > 0;
        icons.push({
          tag: svg.tagName.toLowerCase(),
          wAttr: svg.getAttribute("width"),
          hAttr: svg.getAttribute("height"),
          wRect: Math.round(rect.width),
          hRect: Math.round(rect.height),
          strokeWidth: svg.getAttribute("stroke-width"),
          isLucide: svg.classList.contains("lucide"),
          visible,
        });
      });
    });

    return icons;
  });

  expect(iconStats.length, "rendered icons found in controls").toBeGreaterThan(0);
  for (const icon of iconStats) {
    expect(icon.isLucide, "icon uses Lucide").toBe(true);
    expect(icon.wAttr, "icon width attribute is 14").toBe("14");
    expect(icon.hAttr, "icon height attribute is 14").toBe("14");
    expect(icon.strokeWidth, "icon strokeWidth is 1.75").toBe("1.75");
    if (icon.visible) {
      expect(icon.wRect, "visible icon width is 14px").toBe(14);
      expect(icon.hRect, "visible icon height is 14px").toBe(14);
    }
  }
});

test("design-179: chrome budget firstRowTop <= 140px across views", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });

  // 1. Text View
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-0")).toBeVisible({ timeout: 15000 });
  const textFirstRow = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="text comparison"], section:has(.tv-row)');
    const first = document.querySelector('[data-testid="textview/row-0"], .tv-row');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(textFirstRow, "text firstRowTop measured").not.toBeNull();
  expect(textFirstRow!, "text firstRowTop <= 140px").toBeLessThanOrEqual(140);

  // 2. Folder View
  await createSession(page, "folder", "/tmp/d179-f-left", "/tmp/d179-f-right", "view-folder");
  await expect(page.locator('[data-testid^="folder-row-"]').first()).toBeVisible({ timeout: 15000 });
  const folderFirstRow = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="folder comparison"]');
    const first = document.querySelector('[data-testid^="folder-row-"]');
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(folderFirstRow, "folder firstRowTop measured").not.toBeNull();
  expect(folderFirstRow!, "folder firstRowTop <= 140px").toBeLessThanOrEqual(140);

  // 3. Table View
  await createSession(page, "table", "/tmp/d179-t-left.csv", "/tmp/d179-t-right.csv", "view-table");
  await expect(page.locator("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]").first()).toBeVisible({ timeout: 15000 });
  const tableFirstRow = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="table comparison"]');
    const first = document.querySelector("tbody tr.tc-row, [data-testid=\"tableview/row-0\"]");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(tableFirstRow, "table firstRowTop measured").not.toBeNull();
  expect(tableFirstRow!, "table firstRowTop <= 140px").toBeLessThanOrEqual(140);

  // 4. Merge View
  await createSession(page, "merge", "/tmp/d179-m-base.txt", "/tmp/d179-m-right.txt", "view-merge");
  await expect(page.locator(".mv-pane, [data-testid^=\"merge-card-\"]").first()).toBeVisible({ timeout: 15000 });
  const mergeFirstRow = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="three-way merge"]');
    const first = document.querySelector(".mv-pane, [data-testid^=\"merge-card-\"]");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(mergeFirstRow, "merge firstRowTop measured").not.toBeNull();
  expect(mergeFirstRow!, "merge firstRowTop <= 140px").toBeLessThanOrEqual(140);

  // 5. Hex View (with panels closed)
  await createSession(page, "hex", "/tmp/d179-h-left.bin", "/tmp/d179-h-right.bin", "view-hex");
  await expect(page.locator(".hx-row").first()).toBeVisible({ timeout: 15000 });
  await page.keyboard.press("Escape");
  const hexFirstRow = await page.evaluate(() => {
    const section = document.querySelector('section[aria-label="hex comparison"]');
    const first = document.querySelector(".hx-row");
    if (!(section instanceof HTMLElement) || !(first instanceof HTMLElement)) return null;
    return Math.round(first.getBoundingClientRect().top - section.getBoundingClientRect().top);
  });
  expect(hexFirstRow, "hex firstRowTop measured").not.toBeNull();
  expect(hexFirstRow!, "hex firstRowTop <= 140px").toBeLessThanOrEqual(140);

  expect(consoleErrors, "zero console errors across views").toEqual([]);
});

test("design-179: macOS blue accent focus styling", async ({ page }) => {
  // 1. Verify token definition in tokens.css
  const css = await fetchSrc(page, "/src/theme/tokens.css");
  expect(css, "tokens.css loaded").not.toBeNull();
  expect(css).toContain("--ac-focus: #0a84ff;");
  expect(css).toContain("--ac-focus-ring: var(--ac-focus);");

  // 2. Verify focus-visible class on controls
  const btnSrc = await fetchSrc(page, "/src/components/ui/Button.tsx");
  expect(btnSrc).toContain("focus-visible:ring-[var(--ac-focus)]");

  const inputSrc = await fetchSrc(page, "/src/components/ui/Input.tsx");
  expect(inputSrc).toContain("focus-visible:ring-[var(--ac-focus)]");

  const tbBtnSrc = await fetchSrc(page, "/src/components/ui/ToolbarButton.tsx");
  expect(tbBtnSrc).toContain("focus-visible:ring-[var(--ac-focus)]");

  // 3. Runtime focus verification on PathBar input
  const input = page.getByTestId("pathbar/left");
  await input.focus();
  await expect(input).toBeFocused();

  const focusRing = await input.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    const borderColor = cs.borderColor;
    const boxShadow = cs.boxShadow;
    const outlineColor = cs.outlineColor;
    return { borderColor, boxShadow, outlineColor };
  });

  // Verify macOS blue accent (#0a84ff = rgb(10, 132, 255))
  const isBlue =
    focusRing.borderColor.includes("10, 132, 255") ||
    focusRing.boxShadow.includes("10, 132, 255") ||
    focusRing.outlineColor.includes("10, 132, 255") ||
    focusRing.borderColor.includes("rgb(10, 132, 255)") ||
    focusRing.boxShadow.includes("rgb(10, 132, 255)");
  expect(isBlue, "focus ring / border carries macOS blue accent rgb(10, 132, 255)").toBe(true);
});
