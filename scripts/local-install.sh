#!/bin/sh
# Local install for early feedback (Wave-WE harness, NEW file).
# Builds the Tauri .app bundle and copies it to $APP_DIR/AutoCompare.app
# with a version stamp. Idempotent: safe to re-run (rebuild + replace).
# No src changes. No sudo inside this script — if $APP_DIR is not writable,
# re-run with APP_DIR=$HOME/Applications (no sudo needed).
# Usage: scripts/local-install.sh
# Env: APP_DIR (default /Applications), BUNDLES (default "app")
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DESK="$ROOT/apps/desktop"
VERSION=$(grep '^version = ' "$ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)
SHA=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)
ARCH=$(uname -m)
APP_DIR="${APP_DIR:-/Applications}"
BUNDLES="${BUNDLES:-app}"

T_START=$(date +%s)

TAURI_BIN="$(command -v cargo-tauri 2>/dev/null || echo "$HOME/.cargo/bin/cargo-tauri")"
[ -x "$TAURI_BIN" ] || { echo "MISSING cargo-tauri ($TAURI_BIN): run: cargo install tauri-cli --version '^2'"; exit 1; }
command -v node >/dev/null 2>&1 || { echo "MISSING node (>=20)"; exit 1; }

if [ ! -w "$APP_DIR" ]; then
  echo "APP_DIR_NOT_WRITABLE $APP_DIR (no sudo used by this script)"
  echo "HINT: APP_DIR=\$HOME/Applications $0"
  exit 1
fi

# 0. Frontend deps (fresh worktrees lack node_modules; install once).
if [ ! -x "$DESK/node_modules/.bin/tsc" ]; then
  echo "INSTALL_DEPS $DESK (node_modules missing)"
  if [ -f "$DESK/package-lock.json" ]; then
    (cd "$DESK" && npm ci --no-audit --no-fund)
  else
    (cd "$DESK" && npm install --no-audit --no-fund)
  fi
fi

# 1. Frontend: typecheck + vite build via local binaries (pnpm avoided:
# pnpm v11 pre-build install check fails on ignored esbuild scripts).
"$DESK/node_modules/.bin/tsc" --noEmit -p "$DESK/tsconfig.json"
echo "TSC_OK"
(cd "$DESK" && ./node_modules/.bin/vite build)
CSS=$(ls "$DESK"/dist/assets/*.css 2>/dev/null | head -1)
[ -n "$CSS" ] && [ -s "$CSS" ] || { echo "CSS_MISSING dist/assets/*.css empty"; exit 1; }
echo "FRONTEND_OK $CSS ($(du -h "$CSS" | cut -f1))"

# 2. Tauri bundle (frontend already built; override beforeBuildCommand).
(cd "$DESK" && TAURI_SKIP_UPDATE_CHECK=1 "$TAURI_BIN" tauri build \
  --bundles "$BUNDLES" --ci \
  -c '{"build": {"beforeBuildCommand": ""}}')
APP_SRC=$(ls -d "$DESK"/src-tauri/target/release/bundle/macos/*.app "$ROOT"/target/release/bundle/macos/*.app 2>/dev/null | head -1)
[ -n "$APP_SRC" ] || { echo "APP_BUNDLE_MISSING"; exit 1; }
echo "BUNDLE_OK $APP_SRC"

# 3. Version stamp for traceability.
STAMP="$ROOT/dist/AutoCompare-${VERSION}-macos-${ARCH}.tauri.txt"
mkdir -p "$ROOT/dist"
printf 'tauri %s %s\napp %s\n' "$VERSION" "$SHA" "$APP_SRC" > "$STAMP"
echo "STAMP_OK $STAMP"

# 4. Install: replace previous copy (idempotent), clear quarantine.
APP_NAME=$(basename "$APP_SRC")
mkdir -p "$APP_DIR"
rm -rf "$APP_DIR/$APP_NAME"
cp -R "$APP_SRC" "$APP_DIR/$APP_NAME"
xattr -cr "$APP_DIR/$APP_NAME" 2>/dev/null || true
printf 'tauri %s %s\napp %s\n' "$VERSION" "$SHA" "$APP_DIR/$APP_NAME" > "$STAMP"
echo "INSTALLED $APP_DIR/$APP_NAME"

T_END=$(date +%s)
echo "LOCAL_INSTALL_OK version=$VERSION sha=$SHA arch=$ARCH secs=$((T_END - T_START))"
