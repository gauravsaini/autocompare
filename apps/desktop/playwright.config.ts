import { defineConfig, devices } from "@playwright/test";

const port = Number(process.env.PORT || 1420);

/**
 * AutoCompare v2 Playwright E2E configuration:
 * WebKit (macOS native rendering), 1280x800 @ 2x retina, running against
 * `pnpm exec vite` with the lazy fixture-mock IPC adapter (`VITE_MOCK_IPC=1`)
 * and `VITE_UI_VERSION=v2` by default.
 */
export default defineConfig({
  testDir: "./e2e/v2",
  fullyParallel: true,
  use: {
    baseURL: `http://localhost:${port}`,
  },
  webServer: {
    command: `pnpm exec vite --port ${port}`,
    url: `http://localhost:${port}`,
    reuseExistingServer: !process.env.PORT,
    env: {
      VITE_MOCK_IPC: "1",
      VITE_UI_VERSION: process.env.VITE_UI_VERSION || "v2",
    },
  },
  projects: [
    {
      name: "webkit",
      use: {
        ...devices["Desktop Safari"],
        viewport: { width: 1280, height: 800 },
        deviceScaleFactor: 2,
      },
    },
  ],
});
