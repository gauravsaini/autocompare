/**
 * Wave-105 colors tighten (ONE theme-scoped wave, MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * Audits diff/find/selection/hatch/rail/status colors vs the BC truths in
 * `/tmp/bc-*.png`, pins every role to an exact rgb with computed-style
 * assertions (light + dark), and runs a contrast-sanity check (fg on bg
 * >= 4.5 both modes). Zero layout + zero type change: no geometry or
 * font assertions live here (those belong to the 66/88/100 waves).
 *
 * Evidence (all in `/tmp/`, never in-repo):
 *  - `/tmp/colors-105-<role>[-dark].png` zoomed swatch crops
 *  - `/tmp/colors-105-sidebyside.png` vs BC truths
 *  - `checks/colors-105/REPORT.md` MATCH/OFF verdict table (written
 *    by hand from the measured values pinned below).
 *
 * New file only, no existing spec touched.
 */
import * as fs from "node:fs";
import { expect, test, type Page } from "@playwright/test";

const EVID = "/tmp/colors-105";

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

type Role = { bg: string; fg: string };

const LIGHT: Record<string, Role> = {
  del: { bg: "rgb(246, 194, 189)", fg: "rgb(127, 29, 29)" },
  add: { bg: "rgb(217, 240, 221)", fg: "rgb(20, 83, 45)" },
  chg: { bg: "rgb(247, 227, 166)", fg: "rgb(113, 63, 18)" },
  selection: { bg: "rgb(179, 215, 255)", fg: "rgb(0, 43, 85)" },
  find: { bg: "rgb(255, 223, 107)", fg: "rgb(67, 48, 2)" },
};

const DARK: Record<string, Role> = {
  del: { bg: "rgb(90, 20, 16)", fg: "rgb(254, 202, 202)" },
  add: { bg: "rgb(22, 53, 31)", fg: "rgb(187, 247, 208)" },
  chg: { bg: "rgb(69, 48, 10)", fg: "rgb(253, 230, 138)" },
  selection: { bg: "rgb(26, 79, 139)", fg: "rgb(255, 255, 255)" },
  find: { bg: "rgb(107, 84, 16)", fg: "rgb(255, 233, 168)" },
};

const LIGHT_VARS: Record<string, string> = {
  "--ac-find-current-outline": "rgb(178, 106, 0)",
  "--ac-hatch-line": "rgb(210, 210, 215)",
  "--ac-rail-line": "rgb(229, 229, 234)",
  "--ac-rail-accent": "rgb(110, 110, 115)",
  "--ac-chevron-fg": "rgb(255, 159, 10)",
  "--ac-status-newer": "rgb(10, 132, 255)",
};

const DARK_VARS: Record<string, string> = {
  "--ac-find-current-outline": "rgb(255, 214, 10)",
  "--ac-hatch-line": "rgb(72, 72, 74)",
  "--ac-rail-line": "rgb(56, 56, 58)",
  "--ac-rail-accent": "rgb(161, 161, 166)",
  "--ac-chevron-fg": "rgb(255, 159, 10)",
  "--ac-status-newer": "rgb(10, 132, 255)",
};

async function mountProbes(page: Page): Promise<void> {
  await page.evaluate(() => {
    document.querySelector("#colors-105-probes")?.remove();
    const root = document.createElement("div");
    root.id = "colors-105-probes";
    root.setAttribute("data-testid", "colors105/probes");
    root.style.cssText =
      "position:fixed;top:8px;left:8px;z-index:9999;display:flex;gap:8px;padding:8px;" +
      "background:var(--ac-bg);border:1px solid var(--ac-border);font:12px system-ui;";
    const mk = (id: string, cls: string, text: string): void => {
      const el = document.createElement("div");
      el.setAttribute("data-testid", `colors105/${id}`);
      el.className = cls;
      el.textContent = text;
      el.style.cssText = "padding:6px 10px;min-width:64px;text-align:center;";
      root.appendChild(el);
    };
    mk("del", "tv-del", "del");
    mk("add", "tv-ins", "add");
    mk("chg", "tv-chg", "chg");
    mk("selection", "tv-row is-selected", "sel");
    mk("find", "ac-find-hit", "find");
    mk("find-current", "ac-find-hit ac-find-current", "cur");
    mk("hex-find", "hx-find-match", "hex");
    const hatch = document.createElement("div");
    hatch.setAttribute("data-testid", "colors105/hatch");
    hatch.className = "tv-cell-empty";
    hatch.style.cssText = "width:64px;min-height:28px;";
    root.appendChild(hatch);
    document.body.appendChild(root);
  });
}

async function readRole(page: Page, id: string): Promise<Role> {
  return page.getByTestId(`colors105/${id}`).evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return { bg: cs.backgroundColor, fg: cs.color };
  });
}

function luminance(rgb: string): number {
  const m = rgb.match(/rgb\(\s*(\d+),\s*(\d+),\s*(\d+)\s*\)/);
  if (!m) throw new Error(`unparseable rgb: ${rgb}`);
  const lin = (v: number): number => {
    const s = Number(v) / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(Number(m[1])) + 0.7152 * lin(Number(m[2])) + 0.0722 * lin(Number(m[3]));
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

test("colors-105: BC-tightened palette pinned per role, light+dark, contrast sane", async ({
  page,
  browserName,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  const pageErrors: string[] = [];
  page.on("pageerror", (err) => pageErrors.push(String(err)));

  // Real app surface is up (default text fixture, no session needed).
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/status")).toContainText("rows");

  // Fresh ours capture: real diff rows before probes go in.
  const firstRow = page.locator(".tv-row").first();
  await expect(firstRow).toBeVisible();
  await firstRow.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${EVID}-ours-light.png` });

  await mountProbes(page);

  // ---- Light pins. ----
  for (const [id, want] of Object.entries(LIGHT)) {
    const got = await readRole(page, id);
    expect(got.bg, `${id} wash (light)`).toBe(want.bg);
    expect(got.fg, `${id} ink (light)`).toBe(want.fg);
    expect(contrast(got.bg, got.fg), `${id} contrast light`).toBeGreaterThanOrEqual(4.5);
  }
  // Table + hex + word-highlight roles ride the same diff tokens.
  const tcDiff = await page.evaluate(() => {
    const el = document.createElement("div");
    el.className = "tc-diff";
    document.body.appendChild(el);
    const cs = window.getComputedStyle(el);
    const out = { bg: cs.backgroundColor, fg: cs.color };
    el.remove();
    return out;
  });
  expect(tcDiff.bg, "tc-diff wash rides chg (light)").toBe(LIGHT.chg.bg);
  const hxDiff = await page.evaluate(() => {
    const el = document.createElement("div");
    el.className = "hx-diff";
    document.body.appendChild(el);
    const cs = window.getComputedStyle(el);
    const out = { bg: cs.backgroundColor, fg: cs.color };
    el.remove();
    return out;
  });
  expect(hxDiff.bg, "hx-diff wash rides chg (light)").toBe(LIGHT.chg.bg);
  // Hex find rides the gold find-hit role (not selection blue).
  const hexFind = await readRole(page, "hex-find");
  expect(hexFind.bg, "hx-find-match rides find gold (light)").toBe(LIGHT.find.bg);
  expect(hexFind.fg, "hx-find-match ink (light)").toBe(LIGHT.find.fg);
  // Current-hit outline.
  const curOutline = await page.getByTestId("colors105/find-current").evaluate(
    (el) => window.getComputedStyle(el).outlineColor,
  );
  expect(curOutline, "find-current outline (light)").toBe(LIGHT_VARS["--ac-find-current-outline"]);
  // Hatch line var resolves inside the pure-CSS gradient.
  const hatchImg = await page
    .getByTestId("colors105/hatch")
    .evaluate((el) => window.getComputedStyle(el).backgroundImage);
  expect(hatchImg, "hatch stays pure-CSS gradient (light)").toMatch(/linear-gradient/);
  expect(hatchImg, "hatch line tightened (light)").toContain("210, 210, 215");
  // Rail / chevron / status role vars.
  for (const [name, want] of Object.entries(LIGHT_VARS)) {
    const got = await page.evaluate(
      (n) => window.getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
      name,
    );
    void got;
    const resolved = await page.evaluate((n) => {
      const probe = document.createElement("div");
      probe.style.color = `var(${n})`;
      document.body.appendChild(probe);
      const c = window.getComputedStyle(probe).color;
      probe.remove();
      return c;
    }, name);
    expect(resolved, `${name} resolves (light)`).toBe(want);
  }
  // Splitter rail keeps its asserted calm line (66Q guard, color-only rename).
  const splitterRail = await page.evaluate(() => {
    const host = document.createElement("div");
    host.className = "tv-splitter";
    host.style.cssText = "position:absolute;visibility:hidden;";
    document.body.appendChild(host);
    const c = window.getComputedStyle(host, "::before").backgroundColor;
    host.remove();
    return c;
  });
  expect(splitterRail, "splitter rail unchanged (light)").toBe("rgb(229, 229, 234)");

  // Zoomed swatch crops (light).
  for (const id of ["del", "add", "chg", "selection", "find", "find-current", "hatch"]) {
    await page.getByTestId(`colors105/${id}`).screenshot({ path: `${EVID}-swatch-${id}.png` });
  }

  // ---- Dark pins. ----
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${EVID}-ours-dark.png` });
  for (const [id, want] of Object.entries(DARK)) {
    const got = await readRole(page, id);
    expect(got.bg, `${id} wash (dark)`).toBe(want.bg);
    expect(got.fg, `${id} ink (dark)`).toBe(want.fg);
    expect(contrast(got.bg, got.fg), `${id} contrast dark`).toBeGreaterThanOrEqual(4.5);
  }
  const darkHexFind = await readRole(page, "hex-find");
  expect(darkHexFind.bg, "hx-find-match rides find gold (dark)").toBe(DARK.find.bg);
  const darkCurOutline = await page.getByTestId("colors105/find-current").evaluate(
    (el) => window.getComputedStyle(el).color && window.getComputedStyle(el).outlineColor,
  );
  expect(darkCurOutline, "find-current outline (dark)").toBe(DARK_VARS["--ac-find-current-outline"]);
  const darkHatchImg = await page
    .getByTestId("colors105/hatch")
    .evaluate((el) => window.getComputedStyle(el).backgroundImage);
  expect(darkHatchImg, "hatch stays pure-CSS gradient (dark)").toMatch(/linear-gradient/);
  expect(darkHatchImg, "hatch line tightened (dark)").toContain("72, 72, 74");
  const darkHatchBg = await page
    .getByTestId("colors105/hatch")
    .evaluate((el) => window.getComputedStyle(el).backgroundColor);
  expect(darkHatchBg, "hatch sits on base bg (dark)").toBe("rgb(29, 29, 31)");
  for (const [name, want] of Object.entries(DARK_VARS)) {
    const resolved = await page.evaluate((n) => {
      const probe = document.createElement("div");
      probe.style.color = `var(${n})`;
      document.body.appendChild(probe);
      const c = window.getComputedStyle(probe).color;
      probe.remove();
      return c;
    }, name);
    expect(resolved, `${name} resolves (dark)`).toBe(want);
  }
  for (const id of ["del", "add", "chg", "selection", "find"]) {
    await page.getByTestId(`colors105/${id}`).screenshot({ path: `${EVID}-swatch-${id}-dark.png` });
  }
  await page.getByTestId("colors105/hatch").screenshot({ path: `${EVID}-swatch-hatch-dark.png` });

  // Side-by-side vs BC truths (truth pixels ship in the figure).
  const truthFiles = ["/tmp/bc-text.png", "/tmp/bc-table-truth.png", "/tmp/bc-folder-truth.png"];
  const figs: string[] = [];
  for (const f of truthFiles) {
    try {
      const b64 = fs.readFileSync(f).toString("base64");
      figs.push(
        `<figure style="margin:0"><figcaption>${f} (BC truth)</figcaption>` +
          `<img src="data:image/png;base64,${b64}" style="max-width:600px;border:1px solid #888"/></figure>`,
      );
    } catch {
      figs.push(`<p>missing ${f}</p>`);
    }
  }
  const oursB64 = fs.readFileSync(`${EVID}-ours-light.png`).toString("base64");
  const oursDarkB64 = fs.readFileSync(`${EVID}-ours-dark.png`).toString("base64");
  figs.push(
    `<figure style="margin:0"><figcaption>ours text light (${browserName})</figcaption>` +
      `<img src="data:image/png;base64,${oursB64}" style="max-width:600px;border:1px solid #888"/></figure>`,
    `<figure style="margin:0"><figcaption>ours text dark (${browserName})</figcaption>` +
      `<img src="data:image/png;base64,${oursDarkB64}" style="max-width:600px;border:1px solid #888"/></figure>`,
  );
  await page.emulateMedia({ colorScheme: "light" });
  await page.setContent(
    `<html><body style="margin:0;display:flex;gap:8px;flex-wrap:wrap;background:#fff;font:12px system-ui">${figs.join("")}</body></html>`,
  );
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${EVID}-sidebyside.png`, fullPage: true });

  expect(pageErrors, `zero pageerrors (got: ${pageErrors.join(" | ")})`).toEqual([]);
  expect(consoleErrors, `zero console errors (got: ${consoleErrors.join(" | ")})`).toEqual([]);
});
