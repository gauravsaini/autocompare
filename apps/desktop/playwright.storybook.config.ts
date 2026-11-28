import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.SB_PORT || 6007);

/**
 * Playwright Visual Regression configuration for AutoCompare v2 Storybook.
 * Serves prebuilt `storybook-static` over a local Node static server and runs
 * WebKit visual regression + layout + console checks across all stories.
 */
export default defineConfig({
  testDir: "./e2e/storybook",
  fullyParallel: true,
  snapshotPathTemplate: "{testDir}/__snapshots__/{arg}{ext}",
  expect: {
    toHaveScreenshot: {
      animations: "disabled",
      maxDiffPixelRatio: 0.005,
    },
  },
  use: {
    baseURL: `http://127.0.0.1:${port}`,
  },
  webServer: {
    command: "node ./e2e/storybook/serve-static.mjs",
    url: `http://127.0.0.1:${port}/index.json`,
    reuseExistingServer: !process.env.SB_PORT,
    env: {
      SB_PORT: String(port),
    },
  },
  projects: [
    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
        viewport: { width: 1280, height: 800 },
        deviceScaleFactor: 1,
      },
    },
  ],
});
