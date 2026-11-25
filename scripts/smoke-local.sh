#!/bin/sh
# Local smoke for early feedback (Wave-WE harness, NEW file).
# Launches /Applications/AutoCompare.app (or $APP), waits for boot,
# screenshots OUT_DIR (/tmp/smoke-WE/), asserts app window present,
# then quits. Exit non-zero on fail.
# "Demo diff visible" is verified by a human on the saved screenshots
# (app boots into the demo session; e2e pins demo-banner + 200/644 rows
# in mock mode) — the script guarantees a real window + screenshot exist.
# Usage: scripts/smoke-local.sh
# Env: APP (default /Applications/AutoCompare.app), OUT_DIR (default /tmp/smoke-WE),
#      BOOT_TIMEOUT (default 30s)
set -eu

APP="${APP:-/Applications/AutoCompare.app}"
OUT_DIR="${OUT_DIR:-/tmp/smoke-WE}"
BOOT_TIMEOUT="${BOOT_TIMEOUT:-30}"

T_START=$(date +%s)
mkdir -p "$OUT_DIR"

[ -d "$APP" ] || { echo "SMOKE_FAIL app bundle missing: $APP (run scripts/local-install.sh first)"; exit 1; }
MACOS_BIN=$(ls "$APP/Contents/MacOS/"* 2>/dev/null | head -1)
[ -n "$MACOS_BIN" ] || { echo "SMOKE_FAIL no MacOS binary in $APP"; exit 1; }
echo "SMOKE_APP $APP ($MACOS_BIN)"

# Quit any stale instance so the check is repeatable.
osascript -e 'tell application "AutoCompare" to quit' 2>/dev/null || true
pkill -x AutoCompare 2>/dev/null || true
pkill -x autocompare-desktop 2>/dev/null || true
sleep 1

open "$APP"
echo "SMOKE_OPENED $APP"

# Wait for process + window (poll).
# NOTE: Tauri process name is the lowercase binary (autocompare-desktop),
# not the product name (AutoCompare) — check both.
OK_PROC=0
OK_WIN=0
i=0
while [ "$i" -lt "$BOOT_TIMEOUT" ]; do
  if pgrep -x AutoCompare >/dev/null 2>&1 || pgrep -x autocompare-desktop >/dev/null 2>&1 || pgrep -f "AutoCompare" >/dev/null 2>&1; then
    OK_PROC=1
  fi
  WIN_COUNT=$(osascript -e 'tell application "System Events"
  try
    return count windows of (first process whose name is "AutoCompare")
  end try
  try
    return count windows of (first process whose name is "autocompare-desktop")
  end try
  return 0
end tell' 2>/dev/null | tr -d ' ' || echo 0)
  case "$WIN_COUNT" in ''|*[!0-9]*) WIN_COUNT=0 ;; esac
  if [ "$WIN_COUNT" -ge 1 ] 2>/dev/null; then OK_WIN=1; fi
  if [ "$OK_PROC" = 1 ] && [ "$OK_WIN" = 1 ]; then break; fi
  sleep 1
  i=$((i + 1))
done

SHOT1="$OUT_DIR/smoke-boot-$(date +%Y%m%d-%H%M%S).png"
SHOT2="$OUT_DIR/smoke-window-$(date +%Y%m%d-%H%M%S).png"
screencapture -x "$SHOT1" 2>/dev/null || { echo "SMOKE_FAIL screencapture failed"; exit 1; }
sleep 2
screencapture -x "$SHOT2" 2>/dev/null || { echo "SMOKE_FAIL screencapture failed"; exit 1; }

[ -s "$SHOT1" ] || { echo "SMOKE_FAIL empty screenshot $SHOT1"; exit 1; }
[ -s "$SHOT2" ] || { echo "SMOKE_FAIL empty screenshot $SHOT2"; exit 1; }
echo "SMOKE_SHOTS $SHOT1 ($(du -h "$SHOT1" | cut -f1)) $SHOT2 ($(du -h "$SHOT2" | cut -f1))"

[ "$OK_PROC" = 1 ] || { echo "SMOKE_FAIL process AutoCompare not running after ${BOOT_TIMEOUT}s"; exit 1; }
echo "SMOKE_PROC_OK"
[ "$OK_WIN" = 1 ] || { echo "SMOKE_FAIL no AutoCompare window after ${BOOT_TIMEOUT}s (screenshots kept for review)"; exit 1; }
echo "SMOKE_WINDOW_OK windows=$WIN_COUNT"

echo "NOTE demo-diff rows are verified visually on the screenshots (boot opens demo session)."

# Quit cleanly so reruns are idempotent.
osascript -e 'tell application "AutoCompare" to quit' 2>/dev/null || true
sleep 1
pkill -x AutoCompare 2>/dev/null || true
pkill -x autocompare-desktop 2>/dev/null || true

T_END=$(date +%s)
echo "SMOKE_OK secs=$((T_END - T_START)) shots=$SHOT1,$SHOT2"
