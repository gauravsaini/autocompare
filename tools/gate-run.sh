#!/bin/zsh
# gate-run.sh <apps-desktop-dir> <port> [spec...]
# Runs tsc + vite build + Playwright WebKit against the living v2 E2E suite.
set -e
DIR=$(cd "$1" && pwd -P); PORT=$2; shift 2
TESTDIR="e2e/v2"
ARGS=("$@")
export PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$HOME/ms-playwright}"
cd "$DIR"
./node_modules/.bin/tsc --noEmit && echo TSC:0
./node_modules/.bin/vite build 2>&1 | tail -1
CFG=/tmp/gate-$PORT.config.js
cat > $CFG <<EOJS
const { defineConfig, devices } = require("$DIR/node_modules/@playwright/test");
module.exports = defineConfig({
  testDir: "$DIR/$TESTDIR", fullyParallel: false, workers: 1,
  use: { baseURL: "http://localhost:$PORT" },
  webServer: { command: "./node_modules/.bin/vite --port $PORT --strictPort",
    url: "http://localhost:$PORT", reuseExistingServer: false, cwd: "$DIR",
    env: { VITE_MOCK_IPC: "1", VITE_UI_VERSION: "v2" }, timeout: 120000 },
  projects: [{ name: "webkit", use: { ...devices["Desktop Safari"],
    viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 } }],
});
EOJS
./node_modules/.bin/playwright test --config $CFG "${ARGS[@]}" --workers=1 > /tmp/gate-$PORT.full.log 2>&1; RC=$?; tail -8 /tmp/gate-$PORT.full.log; echo GATE_PLAYWRIGHT_EXIT=$RC; exit $RC
