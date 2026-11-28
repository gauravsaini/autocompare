import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

interface StoryEntry {
  readonly id: string;
  readonly title: string;
  readonly name: string;
  readonly type: string;
  readonly tags?: readonly string[];
}

interface StoryIndexJson {
  readonly entries: Record<string, StoryEntry>;
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const indexPath = path.resolve(__dirname, "../../storybook-static/index.json");

function loadStories(): StoryEntry[] {
  if (!fs.existsSync(indexPath)) {
    throw new Error(
      `Missing ${indexPath}. Run "pnpm storybook:build" before running visual regression tests.`,
    );
  }
  const parsed = JSON.parse(fs.readFileSync(indexPath, "utf8")) as StoryIndexJson;
  return Object.values(parsed.entries)
    .filter((entry) => entry.type === "story")
    .sort((a, b) => a.id.localeCompare(b.id));
}

const STORIES = loadStories();

const LOADING_PLACEHOLDERS = [
  "Loading view…",
  "Loading sessions…",
  "Comparing files…",
  "Comparing folders…",
  "Comparing tables…",
  "Comparing binary streams…",
  "Planning 3-way merge…",
  "Planning 3-way folder merge…",
  "Computing sync plan…",
  "Analyzing picture streams…",
  "Analyzing audio streams…",
];

const DETERMINISTIC_CLOCK_INIT = () => {
  Date.prototype.toLocaleTimeString = () => "02:30:00 PM";
  Date.prototype.toLocaleDateString = () => "Nov 28, 2026";
  Date.prototype.toLocaleString = () => "Nov 28, 02:30 PM";
};

async function openStoryAndVerify(
  page: Page,
  story: StoryEntry,
  theme: "light" | "dark",
  layoutDensity: "standard" | "compact" = "standard",
): Promise<void> {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];
  const failedAssets: string[] = [];

  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });
  page.on("pageerror", (err) => pageErrors.push(String(err)));
  page.on("response", (resp) => {
    if (resp.status() >= 400 && !resp.url().includes("favicon")) {
      failedAssets.push(`${resp.status()} ${resp.url()}`);
    }
  });

  await page.addInitScript(DETERMINISTIC_CLOCK_INIT);

  const url = `/iframe.html?id=${encodeURIComponent(story.id)}&viewMode=story&globals=theme:${theme};layout:${layoutDensity}`;
  await page.goto(url, { waitUntil: "domcontentloaded" });

  // Wait for Storybook to mount and exit preparing phase
  await page.waitForFunction(
    () =>
      document.body.classList.contains("sb-show-main") &&
      !document.body.classList.contains("sb-show-preparing-story") &&
      !document.body.classList.contains("sb-show-preparing-docs"),
    undefined,
    { timeout: 15000 },
  );

  // Surface any Storybook error display cleanly
  const errorBox = page.locator("#error-stack, .sb-errordisplay");
  if ((await errorBox.count()) > 0 && (await errorBox.first().isVisible())) {
    const errTxt = await errorBox.first().innerText();
    throw new Error(`Storybook error overlay on ${story.id}: ${errTxt}`);
  }

  // Wait for async view loading states to resolve
  await page.waitForFunction(
    (placeholders) => {
      const root = document.getElementById("storybook-root");
      if (!root) return false;
      const txt = root.innerText || "";
      return !placeholders.some((p) => txt.includes(p));
    },
    LOADING_PLACEHOLDERS,
    { timeout: 15000 },
  );

  // Ensure fonts and images are fully loaded, then settle layout
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(
      Array.from(document.images).map((img) =>
        img.complete
          ? Promise.resolve()
          : new Promise<void>((resolve) => {
              img.addEventListener("load", () => resolve(), { once: true });
              img.addEventListener("error", () => resolve(), { once: true });
            }),
      ),
    );
    await new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())));
  });

  // Intentionally empty stories (e.g. EmptyHidden) skip non-empty DOM check
  const isEmptyStory = story.id.endsWith("--empty-hidden");
  if (!isEmptyStory) {
    const hasContent = await page.evaluate(() => {
      const root = document.getElementById("storybook-root");
      return Boolean(root && root.children.length > 0 && root.getBoundingClientRect().height > 0);
    });
    expect(hasContent, `${story.id} (${theme}) rendered empty root`).toBe(true);
  }

  // Assert no broken images
  const brokenImages = await page.evaluate(() =>
    Array.from(document.images)
      .filter((img) => !img.complete || img.naturalWidth === 0)
      .map((img) => img.alt || img.src.slice(0, 64)),
  );
  expect(brokenImages, `${story.id} (${theme}) has broken images`).toEqual([]);

  // Assert zero horizontal overflow at current viewport
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow, `${story.id} (${theme}) horizontal overflow`).toBeLessThanOrEqual(1);

  expect(consoleErrors, `${story.id} (${theme}) console.error`).toEqual([]);
  expect(pageErrors, `${story.id} (${theme}) pageerror`).toEqual([]);
  expect(failedAssets, `${story.id} (${theme}) failed requests`).toEqual([]);
}

test.describe("Storybook Visual Regression (Light & Dark)", () => {
  for (const story of STORIES) {
    for (const theme of ["light", "dark"] as const) {
      test(`${story.id} [${theme}]`, async ({ page }) => {
        await openStoryAndVerify(page, story, theme);
        await expect(page).toHaveScreenshot(`${story.id}--${theme}.png`);
      });
    }
  }
});

const RESPONSIVE_CRITICAL_IDS = [
  "v2-shell-v2app--home-launcher",
  "v2-views-v2textcompare--side-by-side-demo-diff",
  "v2-views-v2foldercompare--default-tree",
  "v2-views-v2mergecompare--unresolved-conflict",
  "v2-views-v2mediacompare--picture-slider-wipe",
  "v2-modals-v2newsessionmodal--ten-plugin-grid",
] as const;

test.describe("Storybook Responsive & Compact Density Regression", () => {
  for (const storyId of RESPONSIVE_CRITICAL_IDS) {
    const story = STORIES.find((s) => s.id === storyId);
    if (!story) continue;

    test(`${story.id} [compact 960x700]`, async ({ page }) => {
      await page.setViewportSize({ width: 960, height: 700 });
      await openStoryAndVerify(page, story, "light", "compact");
      await expect(page).toHaveScreenshot(`${story.id}--compact-960x700.png`);
    });
  }
});
