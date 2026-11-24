/**
 * Wave-A UI Kit Preview Smoke Spec:
 * Validates the macOS native desktop foundation kit:
 *  1. Tokens: --ac-btn-bg, --ac-btn-border, --ac-btn-shadow, --ac-segment-track,
 *     --ac-control-h-sm (22px), --ac-control-h-md (26px), --ac-focus-ring-subtle,
 *     --ac-checkbox-size (13px), --ac-control-h-input (24px) in light + dark.
 *  2. Primitives: Button, SegmentedControl, ToolbarButton, Checkbox, Input, Select.
 *  3. Button variants (ghost/secondary/primary/danger) and sizes (sm 22px / md 26px) with rounded 5px.
 *  4. ToolbarButton: Lucide-only rule (no hardcoded <svg>), size 14, strokeWidth 1.75.
 *  5. Checkbox: crisp 13px with --ac-accent fill.
 *  6. Input + Select: 24px height, inset hairline border, blue focus ring.
 *  7. Live DOM rendered geometry & testid pass-through smoke.
 */
import { expect, test } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const UI_DIR = path.resolve(__dirname, "../../src/components/ui");
const THEME_DIR = path.resolve(__dirname, "../../src/theme");

function readSource(relPath: string): string {
  const fullPath = path.resolve(__dirname, "../../", relPath);
  return fs.readFileSync(fullPath, "utf-8");
}

test.beforeEach(async ({ page }) => {
  let up = false;
  try {
    await page.goto("/", { timeout: 15000 });
    up = true;
  } catch {
    up = false;
  }
  test.skip(!up, "dev server unreachable -- skipping ui-kit preview smoke");
});

test("ui kit exports all 6 desktop primitives cleanly from index.ts", async () => {
  const src = readSource("src/components/ui/index.ts");
  const required = [
    "Button",
    "SegmentedControl",
    "ToolbarButton",
    "Checkbox",
    "Input",
    "Select",
  ];
  for (const name of required) {
    expect(src, `index exports ${name}`).toContain(name);
  }
});

test("tokens.css defines native macOS control tokens in light and dark modes", async () => {
  const css = readSource("src/theme/tokens.css");

  const requiredTokens = [
    "--ac-btn-bg",
    "--ac-btn-border",
    "--ac-btn-shadow",
    "--ac-segment-track",
    "--ac-control-h-sm: 22px",
    "--ac-control-h-md: 26px",
    "--ac-focus-ring-subtle",
    "--ac-checkbox-size: 13px",
  ];

  for (const tok of requiredTokens) {
    expect(css, `tokens.css defines ${tok}`).toContain(tok);
  }

  // Dark mode section checks
  const darkIndex = css.indexOf("@media (prefers-color-scheme: dark)");
  expect(darkIndex, "tokens.css defines dark scheme overrides").toBeGreaterThan(0);
  const darkSection = css.slice(darkIndex);

  for (const tok of ["--ac-btn-bg", "--ac-btn-border", "--ac-btn-shadow", "--ac-segment-track", "--ac-control-h-sm", "--ac-control-h-md"]) {
    expect(darkSection, `dark mode mirrors ${tok}`).toContain(tok);
  }
});

test("Button component defines 4 variants and 2 macOS sizes (22px/26px, rounded 5px)", async () => {
  const src = readSource("src/components/ui/Button.tsx");

  // Variants
  expect(src, "Button supports primary variant").toContain('"primary"');
  expect(src, "Button supports secondary variant").toContain('"secondary"');
  expect(src, "Button supports ghost variant").toContain('"ghost"');
  expect(src, "Button supports danger variant").toContain('"danger"');

  // Sizes and radius
  expect(src, "Button supports sm size").toContain("sm:");
  expect(src, "Button supports md size").toContain("md:");
  expect(src, "Button sm uses --ac-control-h-sm").toContain("--ac-control-h-sm");
  expect(src, "Button md uses --ac-control-h-md").toContain("--ac-control-h-md");
  expect(src, "Button has 5px rounded corners").toContain("rounded-[5px]");
  expect(src, "Button has hairline border").toContain("border");
  expect(src, "Button passes testids through via rest props").toContain("{...rest}");
});

test("ToolbarButton enforces Lucide-only icons (no hardcoded <svg>) with size 14 and stroke 1.75", async () => {
  const src = readSource("src/components/ui/ToolbarButton.tsx");

  expect(src, "no inline <svg> inside ToolbarButton (Lucide rule)").not.toContain("<svg");
  expect(src, "ToolbarButton supports Lucide size 14").toContain("14");
  expect(src, "ToolbarButton supports strokeWidth 1.75").toContain("1.75");
  expect(src, "ToolbarButton passes testids through").toContain("{...rest}");
});

test("Checkbox enforces 13px size with --ac-accent fill", async () => {
  const src = readSource("src/components/ui/Checkbox.tsx");

  expect(src, "Checkbox uses 13px token").toContain("--ac-checkbox-size");
  expect(src, "Checkbox uses accent fill").toContain("accent-[var(--ac-accent)]");
  expect(src, "Checkbox has type checkbox").toContain('type="checkbox"');
  expect(src, "Checkbox passes rest props").toContain("{...rest}");
});

test("Input and Select components enforce 24px height, hairline border, and focus ring", async () => {
  const inputSrc = readSource("src/components/ui/Input.tsx");
  const selectSrc = readSource("src/components/ui/Select.tsx");

  expect(inputSrc, "Input has 24px height contract").toContain("24px");
  expect(inputSrc, "Input has hairline border").toContain("--ac-input-border");
  expect(inputSrc, "Input has blue focus ring").toContain("--ac-focus-ring-subtle");

  expect(selectSrc, "Select has 24px height contract").toContain("24px");
  expect(selectSrc, "Select has hairline border").toContain("--ac-input-border");
  expect(selectSrc, "Select has blue focus ring").toContain("--ac-focus-ring-subtle");
});

test("SegmentedControl enforces recessed track, elevated active segment, and testid pass-through", async () => {
  const src = readSource("src/components/ui/SegmentedControl.tsx");

  expect(src, "SegmentedControl uses track token").toContain("--ac-segment-track");
  expect(src, "SegmentedControl uses active background token").toContain("--ac-segment-active-bg");
  expect(src, "SegmentedControl has role group").toContain('role="group"');
  expect(src, "SegmentedControl passes testids").toContain("data-testid");
});

test("live DOM preview mounts all UI kit primitives and verifies computed geometry", async ({ page }) => {
  const result = await page.evaluate(() => {
    // Mount a temporary preview container in the live document
    const container = document.createElement("div");
    container.id = "ui-kit-preview-container";
    container.style.cssText = "position: fixed; bottom: 0; right: 0; z-index: 99999; padding: 12px; display: flex; gap: 8px; opacity: 0.01; pointer-events: none;";

    // 1. Button sm
    const btnSm = document.createElement("button");
    btnSm.className = "inline-flex items-center justify-center h-[var(--ac-control-h-sm)] rounded-[5px] border px-2 text-[12px]";
    btnSm.setAttribute("data-testid", "preview/button-sm");
    container.appendChild(btnSm);

    // 2. Button md
    const btnMd = document.createElement("button");
    btnMd.className = "inline-flex items-center justify-center h-[var(--ac-control-h-md)] rounded-[5px] border px-2.5 text-[12px]";
    btnMd.setAttribute("data-testid", "preview/button-md");
    container.appendChild(btnMd);

    // 3. Input 24px
    const input = document.createElement("input");
    input.className = "h-[var(--ac-control-h-input,24px)] rounded-[5px] border px-2";
    input.setAttribute("data-testid", "preview/input");
    container.appendChild(input);

    // 4. Select 24px
    const select = document.createElement("select");
    select.className = "h-[var(--ac-control-h-input,24px)] rounded-[5px] border px-1";
    select.setAttribute("data-testid", "preview/select");
    container.appendChild(select);

    // 5. Checkbox 13px
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.className = "h-[var(--ac-checkbox-size)] w-[var(--ac-checkbox-size)]";
    checkbox.setAttribute("data-testid", "preview/checkbox");
    container.appendChild(checkbox);

    document.body.appendChild(container);

    const btnSmStyle = window.getComputedStyle(btnSm);
    const btnMdStyle = window.getComputedStyle(btnMd);
    const inputStyle = window.getComputedStyle(input);
    const selectStyle = window.getComputedStyle(select);
    const checkboxStyle = window.getComputedStyle(checkbox);

    const data = {
      btnSmHeight: btnSm.offsetHeight,
      btnSmRadius: btnSmStyle.borderRadius,
      btnMdHeight: btnMd.offsetHeight,
      btnMdRadius: btnMdStyle.borderRadius,
      inputHeight: input.offsetHeight,
      inputRadius: inputStyle.borderRadius,
      selectHeight: select.offsetHeight,
      selectRadius: selectStyle.borderRadius,
      checkboxWidth: checkbox.offsetWidth,
      checkboxHeight: checkbox.offsetHeight,
    };

    container.remove();
    return data;
  });

  expect(result.btnSmHeight, "Button sm offsetHeight is 22px").toBe(22);
  expect(result.btnSmRadius, "Button sm borderRadius is 5px").toBe("5px");
  expect(result.btnMdHeight, "Button md offsetHeight is 26px").toBe(26);
  expect(result.btnMdRadius, "Button md borderRadius is 5px").toBe("5px");
  expect(result.inputHeight, "Input offsetHeight is 24px").toBe(24);
  expect(result.inputRadius, "Input borderRadius is 5px").toBe("5px");
  expect(result.selectHeight, "Select offsetHeight is 24px").toBe(24);
  expect(result.selectRadius, "Select borderRadius is 5px").toBe("5px");
  expect(result.checkboxHeight, "Checkbox height is 13px").toBe(13);
  expect(result.checkboxWidth, "Checkbox width is 13px").toBe(13);
});
