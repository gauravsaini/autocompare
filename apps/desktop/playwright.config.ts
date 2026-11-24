import { defineConfig, devices } from "@playwright/test";

/**
 * Wave 0 e2e scaffold: WebKit-only (macOS target), deviceScaleFactor 2 for
 * retina screenshots, 1280x800 to match the Tauri window geometry in
 * `src-tauri/tauri.conf.json`. Runs against `npm run dev` with the
 * fixture-mock IPC adapter (`VITE_MOCK_IPC=1`) so no Tauri binary is needed.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  use: {
    baseURL: "http://localhost:1420",
  },
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: true,
    env: {
      VITE_MOCK_IPC: "1",
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
