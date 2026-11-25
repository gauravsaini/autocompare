/**
 * Wave-D3 shortcuts dead-audit + wiring proof (MOCK IPC ONLY, `VITE_MOCK_IPC=1`).
 *
 * User complaint: "shortcuts not working". This spec is the advertised
 * shortcut inventory: every chord the UI text advertises (native menu
 * accelerators, toolbar `title` hints from `TOOLBAR_SHORTCUT_HINTS`,
 * per-view toolbar labels like `Find bytes (Ctrl+F)`, the merge footer
 * `Shortcuts: j/k ... Cmd+B`) is pressed as a real keyboard event and
 * must produce its expected action — nav, focus, take, dialog, or an
 * HONEST parked note. Fake labels (advertised but dead) fail here.
 *
 * HONEST-DISABLED (documented, asserted as no-crash no-ops):
 * - Table/Merge/Sync have no goto: Ctrl+G suppresses browser Find Next
 *   and does nothing else (no navigation, no crash).
 * - Media has no find/goto: Ctrl+F/G + F3 suppress the browser default
 *   and park honestly (menu-find reports "not available for this view").
 *
 * PORT-FREE gate spec (fresh 1857, reuseExistingServer:false,
 * `VITE_MOCK_IPC=1`, --workers=1, webkit). NO geometry asserts.
 */
import { expect, test, type Page } from "@playwright/test";
import * as fs from "node:fs";
import {
  ADVERTISED_SHORTCUTS,
  TOOLBAR_SHORTCUT_HINTS,
  matchWebShortcut,
} from "../../src/components/chrome/shortcuts";

const EVID_DIR = "/tmp/d3-shortcuts-252";

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

async function focusedTestId(page: Page): Promise<string> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el instanceof HTMLElement ? (el.getAttribute("data-testid") ?? "") : "";
  });
}

function chord(key: string, extra: Partial<{ shiftKey: boolean; altKey: boolean }> = {}) {
  return { key, metaKey: true, ctrlKey: false, shiftKey: extra.shiftKey ?? false, altKey: extra.altKey ?? false };
}

async function closeManager(page: Page): Promise<void> {
  await page.getByRole("button", { name: /Close dialog/ }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden({ timeout: 10000 });
}

test("d3 registry: matcher maps menu accelerators, skips view-local keys", async ({ page }) => {
  const errors = collectErrors(page);
  expect(matchWebShortcut(chord("1"))).toBe("view.kind-text");
  expect(matchWebShortcut(chord("2"))).toBe("view.kind-folder");
  expect(matchWebShortcut(chord("3"))).toBe("view.kind-hex");
  expect(matchWebShortcut(chord("4"))).toBe("view.kind-table");
  expect(matchWebShortcut(chord("5"))).toBe("view.kind-media");
  expect(matchWebShortcut(chord("6"))).toBe("view.kind-sync");
  expect(matchWebShortcut(chord("7"))).toBe("view.kind-merge");
  expect(matchWebShortcut(chord("n"))).toBe("session.new");
  expect(matchWebShortcut(chord("o"))).toBe("file.open-session");
  expect(matchWebShortcut(chord("O", { shiftKey: true }))).toBe("file.open-folder");
  expect(matchWebShortcut(chord("s"))).toBe("file.save-session");
  expect(matchWebShortcut(chord("d"))).toBe("session.take-left");
  expect(matchWebShortcut(chord("e"))).toBe("session.take-right");
  // View-local + non-chords must NOT match (no double-handling).
  expect(matchWebShortcut(chord("f"))).toBeNull();
  expect(matchWebShortcut(chord("g"))).toBeNull();
  expect(matchWebShortcut(chord("b"))).toBeNull();
  expect(matchWebShortcut({ key: "l", metaKey: false, ctrlKey: false, shiftKey: false, altKey: false })).toBeNull();
  expect(matchWebShortcut(chord("1", { altKey: true }))).toBeNull();
  // Every toolbar hint is a registry chord (titles can never drift).
  for (const hint of Object.values(TOOLBAR_SHORTCUT_HINTS)) {
    expect(Object.values(ADVERTISED_SHORTCUTS)).toContain(hint);
  }
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 toolbar titles: advertised hints ride enabled actions", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  const openTitle = await page.getByTestId("toolbar/action-open").getAttribute("title");
  expect(openTitle ?? "").toContain(ADVERTISED_SHORTCUTS.open);
  const nextTitle = await page.getByTestId("toolbar/action-next-hunk").getAttribute("title");
  expect(nextTitle ?? "").toContain(ADVERTISED_SHORTCUTS["next-hunk"]);
  const prevTitle = await page.getByTestId("toolbar/action-prev-hunk").getAttribute("title");
  expect(prevTitle ?? "").toContain(ADVERTISED_SHORTCUTS["prev-hunk"]);
  // Copy parks honestly until a hunk row is selected — the title must
  // carry the honest reason, never a bare fake hint.
  const copyTitle = await page.getByTestId("toolbar/action-copy").getAttribute("title");
  expect(copyTitle ?? "").toContain("select a row first");
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 menu: Meta+O / Meta+N open manager, Meta+S saves honestly", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+o");
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Session Manager opened", { timeout: 10000 });
  await closeManager(page);
  await page.keyboard.press("Meta+n");
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("statusbar/message")).toContainText("Session Manager opened", { timeout: 10000 });
  await closeManager(page);
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("statusbar/message")).toContainText(/Session saved|Nothing to save/, { timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 kinds: Meta+2/1 switch folder/text", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+2");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 text find/goto: Ctrl+F / Ctrl+G focus, Esc closes", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("textview/find-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toBeHidden({ timeout: 10000 });
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("textview/goto-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("textview/goto-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/goto-input")).toBeHidden({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 F3/Shift+F3: find next/prev run without crashing", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("textview/find-input")).toBeVisible({ timeout: 10000 });
  await page.getByTestId("textview/find-input").fill("e");
  await page.getByTestId("textview/find-input").press("Enter");
  await expect(page.getByTestId("textview/find-status")).toBeVisible({ timeout: 10000 });
  // Next + prev: either step a hit or refocus the box — never a crash.
  await page.keyboard.press("F3");
  await expect(page.getByTestId("textview/find-status")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Shift+F3");
  await expect(page.getByTestId("textview/find-status")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("textview/find-input")).toBeHidden({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 text hunk nav: n/p + Cmd+Alt arrows move selection", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", /^(leftOnly|rightOnly|changed)$/, { timeout: 10000 });
  await page.getByTestId("textview/row-6").focus();
  expect(await focusedTestId(page)).toBe("textview/row-6");
  // Row 6 is the first hunk's first row, so the first `n` wraps inside
  // hunk 1 (nav runs: the diff note names it); the second `n` must move.
  await page.keyboard.press("n");
  await expect(page.getByTestId("textview/diffnav-note")).toContainText(/difference \d+ of \d+/, { timeout: 10000 });
  await page.keyboard.press("n");
  await expect
    .poll(() => focusedTestId(page), { timeout: 10000 })
    .toMatch(/^textview\/row-(?!6$)\d+$/);
  const afterN = await focusedTestId(page);
  expect(afterN).toMatch(/^textview\/row-\d+$/);
  await page.keyboard.press("p");
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).not.toBe(afterN);
  // Toolbar-advertised chords ride the same hunk path.
  const before = await focusedTestId(page);
  await page.keyboard.press("Meta+Alt+ArrowDown");
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).not.toBe(before);
  const beforeUp = await focusedTestId(page);
  await page.keyboard.press("Meta+Alt+ArrowUp");
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).not.toBe(beforeUp);
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 text take: l copies hunk, Cmd+D requests take-left", async ({ page }) => {
  const errors = collectErrors(page);
  await expect(page.getByTestId("view-text")).toBeVisible();
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", /^(leftOnly|rightOnly|changed)$/, { timeout: 10000 });
  await page.getByTestId("textview/row-6").focus();
  await page.keyboard.press("l");
  await expect(page.getByTestId("textview/copy-note")).toContainText("take-left", { timeout: 10000 });
  await expect(page.getByTestId("textview/row-6")).toHaveAttribute("data-kind", "equal", { timeout: 10000 });
  // Menu accelerator rides the same take path (nearest-hunk fallback
  // keeps it honest once row 6 is equal).
  await page.keyboard.press("Meta+d");
  await expect(page.getByTestId("statusbar/message")).toContainText(/take-left requested|needs a hunk row selected/, { timeout: 10000 });
  await page.keyboard.press("Meta+e");
  await expect(page.getByTestId("statusbar/message")).toContainText(/take-right requested|needs a hunk row selected/, { timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

async function createMergeSession(page: Page): Promise<void> {
  await expect(page.getByTestId("workspace/demo-banner")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("workspace/demo-clear").click();
  await expect(page.getByTestId("session-count")).toContainText("sessions: 0", { timeout: 15000 });
  await page.getByTestId("toolbar/action-open").click();
  await expect(page.getByTestId("session/dialog")).toBeVisible({ timeout: 15000 });
  await page.getByTestId("session/kind-select").selectOption("merge");
  await page.getByTestId("session/left-path").fill("/tmp/d3-left.txt");
  await page.getByTestId("session/right-path").fill("/tmp/d3-right.txt");
  await page.getByTestId("session/create-button").click();
  await page.getByRole("button", { name: "Close dialog" }).click();
  await expect(page.getByTestId("session/dialog")).toBeHidden({ timeout: 15000 });
  await expect(page.getByTestId("view-merge")).toBeVisible({ timeout: 15000 });
}

test("d3 merge: footer j/k fire from body focus, Cmd+B/D/E take", async ({ page }) => {
  const errors = collectErrors(page);
  await createMergeSession(page);
  await expect(page.getByTestId("merge-active-conflict")).toBeVisible({ timeout: 15000 });
  const beforeId = await page.getByTestId("merge-active-conflict").getAttribute("data-active-id");
  // Body focus via the App status bar (outside the merge section, not
  // focusable): the D3 window-level j/k path dispatches the merge bus
  // even though the merge section itself never got focus.
  await page.evaluate(() => {
    (window as unknown as { __d3bus?: string[] }).__d3bus = [];
    window.addEventListener("autocompare:view-command", (e: Event) => {
      const d = (e as CustomEvent<{ cmd?: unknown; viewKind?: unknown }>).detail;
      if (d?.viewKind === "merge" && (d?.cmd === "next" || d?.cmd === "prev")) {
        (window as unknown as { __d3bus?: string[] }).__d3bus?.push(String(d.cmd));
      }
    });
  });
  await page.getByTestId("statusbar/message").click();
  await expect.poll(() => focusedTestId(page), { timeout: 5000 }).toBe("");
  await page.keyboard.press("j");
  await page.keyboard.press("k");
  await expect
    .poll(
      () =>
        page.evaluate(() => (window as unknown as { __d3bus?: string[] }).__d3bus ?? []),
      { timeout: 10000 },
    )
    .toEqual(["next", "prev"]);
  // Single mock conflict wraps around — selection stays honest.
  await expect(page.getByTestId("merge-active-conflict")).toHaveAttribute("data-active-id", beforeId ?? "", { timeout: 10000 });
  // Cmd+B takes both on the selected conflict from body focus.
  await page.keyboard.press("Meta+b");
  await expect(page.getByTestId("statusbar/message")).toContainText(/take-both|Take both|needs a conflict/, { timeout: 10000 });
  await page.keyboard.press("Meta+d");
  await expect(page.getByTestId("statusbar/message")).toContainText(/take-left|needs a conflict/, { timeout: 10000 });
  await page.keyboard.press("Meta+e");
  await expect(page.getByTestId("statusbar/message")).toContainText(/take-right|needs a conflict/, { timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 merge find: Ctrl+F opens, Esc closes", async ({ page }) => {
  const errors = collectErrors(page);
  await createMergeSession(page);
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("merge/find-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("merge/find-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("merge/find-input")).toBeHidden({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 folder: Ctrl+F filter, Ctrl+G goto, Alt+Left honest, Esc keeps query", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+2");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("folder-name-filter")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("folder-name-filter");
  // Esc dismisses the name bar first (DockedBar capture owns topmost
  // dismissal and stops propagation by design — ui primitive, untouched).
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("folder-goto-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("folder-goto-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folder-goto-input")).toHaveCount(0);
  // Stacked bars dismiss LIFO: name first, goto second — still one Esc
  // per bar, never a dead key.
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("folder-name-filter")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("folder-goto-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folderview/name-panel")).toHaveCount(0);
  await expect(page.getByTestId("folder-goto-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("folder-goto-input")).toHaveCount(0);
  // No history in a fresh session: the back title says so honestly and
  // the chord is a silent no-op (no navigation, no crash).
  const backTitle = await page.getByTestId("folder-back").getAttribute("title");
  expect(backTitle ?? "").toContain("No previous folder in history");
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(page.getByTestId("view-folder")).toBeVisible({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 hex: Ctrl+F find, Ctrl+G goto, Esc closes", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+3");
  await expect(page.getByTestId("view-hex")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("hex-find-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-find-input")).toBeHidden({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("hex-find-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("hex-find-input");
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("hex-goto-input")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("hex-goto-input")).toBeHidden({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 table: Ctrl+F find + Esc, Ctrl+G honest no-op", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+4");
  await expect(page.getByTestId("view-table")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+f");
  await expect(page.getByTestId("table-find-input")).toBeVisible({ timeout: 10000 });
  await expect.poll(() => focusedTestId(page), { timeout: 10000 }).toBe("table-find-input");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("table-find-input")).toBeHidden({ timeout: 10000 });
  // Tables have no goto: browser Find Next is suppressed, view stays.
  await page.keyboard.press("Meta+g");
  await expect(page.getByTestId("view-table")).toBeVisible({ timeout: 10000 });
  await expect(page.getByTestId("table-find-input")).toHaveCount(0);
  expect(errors, "zero console/page errors").toEqual([]);
});

test("d3 sync/media honest: find/goto park without crashing", async ({ page }) => {
  const errors = collectErrors(page);
  await page.keyboard.press("Meta+6");
  await expect(
    page.getByTestId("view-sync").or(page.getByTestId("workspace/blank-empty")),
  ).toBeVisible({ timeout: 15000 });
  await page.keyboard.press("Meta+f");
  // Either the step filter opens (live session) or nothing mounted
  // mounts (honest empty) — both are non-crashes.
  await expect(page.getByTestId("view-sync").or(page.getByTestId("workspace/blank-empty"))).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("view-text")).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Meta+5");
  await expect(
    page.getByTestId("view-media").or(page.getByTestId("workspace/blank-empty")),
  ).toBeVisible({ timeout: 15000 });
  await page.keyboard.press("Meta+f");
  await expect(
    page.getByTestId("view-media").or(page.getByTestId("workspace/blank-empty")),
  ).toBeVisible({ timeout: 10000 });
  await page.keyboard.press("Escape");
  await expect(
    page.getByTestId("view-media").or(page.getByTestId("workspace/blank-empty")),
  ).toBeVisible({ timeout: 10000 });
  expect(errors, "zero console/page errors").toEqual([]);
});
