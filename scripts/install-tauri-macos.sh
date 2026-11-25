#!/bin/sh
# Tauri macOS early-feedback build + install for AutoCompare desktop.
# Reusable single entry-point so humans and agents never hand-roll steps.
# Usage: scripts/install-tauri-macos.sh [--install] [--open]
#   (no flags)  typecheck + vite build + Tauri .app bundle only
#   --install   also copy AutoCompare.app to $APP_DIR (default /Applications)
#   --open      also launch the app after build/install
# Env: APP_DIR (default /Applications), BUNDLES (default "app"; e.g. "app,dmg")
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DESK="$ROOT/apps/desktop"
VERSION=$(grep '^version = ' "$ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)
SHA=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)
ARCH=$(uname -m)
APP_DIR="${APP_DIR:-/Applications}"
BUNDLES="${BUNDLES:-app}"

TAURI_BIN="$(command -v cargo-tauri 2>/dev/null || echo "$HOME/.cargo/bin/cargo-tauri")"
[ -x "$TAURI_BIN" ] || { echo "MISSING cargo-tauri ($TAURI_BIN)"; exit 1; }

# 0. Frontend deps: fresh clones/worktrees lack node_modules; install once.
# NOTE: npm (not pnpm) — apps/desktop tracks package-lock.json, and pnpm v11's
# pre-build install check fails on ignored esbuild scripts (see step 1 NOTE).
if [ ! -x "$DESK/node_modules/.bin/tsc" ]; then
  echo "INSTALL_DEPS $DESK (node_modules missing)"
  if [ -f "$DESK/package-lock.json" ]; then
    (cd "$DESK" && npm ci --no-audit --no-fund)
  else
    (cd "$DESK" && npm install --no-audit --no-fund)
  fi
fi

# 1. Frontend: typecheck + vite build via node_modules binaries directly.
# NOTE: `pnpm build` is deliberately avoided — pnpm v11's pre-build install
# check fails on ignored esbuild scripts (ERR_PNPM_IGNORED_BUILDS).
"$DESK/node_modules/.bin/tsc" --noEmit -p "$DESK/tsconfig.json"
echo "TSC_OK"
(cd "$DESK" && ./node_modules/.bin/vite build)
CSS=$(ls "$DESK"/dist/assets/*.css 2>/dev/null | head -1)
[ -n "$CSS" ] && [ -s "$CSS" ] || { echo "CSS_MISSING dist/assets/*.css empty"; exit 1; }
echo "FRONTEND_OK $CSS ($(du -h "$CSS" | cut -f1))"

# 2. Tauri bundle. beforeBuildCommand is overridden to empty because the
# frontend is already built above (and `pnpm build` would fail, see NOTE).
(cd "$DESK" && TAURI_SKIP_UPDATE_CHECK=1 "$TAURI_BIN" tauri build \
  --bundles "$BUNDLES" --ci \
  -c '{"build": {"beforeBuildCommand": ""}}')
APP_SRC=$(ls -d "$DESK"/src-tauri/target/release/bundle/macos/*.app "$ROOT"/target/release/bundle/macos/*.app 2>/dev/null | head -1)
[ -n "$APP_SRC" ] || { echo "APP_BUNDLE_MISSING"; exit 1; }
echo "BUNDLE_OK $APP_SRC"

# 3. Stamp for traceability (version + git sha of what was bundled).
STAMP="$ROOT/dist/AutoCompare-${VERSION}-macos-${ARCH}.tauri.txt"
mkdir -p "$ROOT/dist"
printf 'tauri %s %s\napp %s\n' "$VERSION" "$SHA" "$APP_SRC" > "$STAMP"
echo "STAMP_OK $STAMP"

if [ "${1:-}" = "--install" ] || [ "${2:-}" = "--install" ]; then
  APP_NAME=$(basename "$APP_SRC")
  mkdir -p "$APP_DIR"
  rm -rf "$APP_DIR/$APP_NAME"
  cp -R "$APP_SRC" "$APP_DIR/$APP_NAME"
  xattr -cr "$APP_DIR/$APP_NAME" 2>/dev/null || true
  printf 'tauri %s %s\napp %s\n' "$VERSION" "$SHA" "$APP_DIR/$APP_NAME" > "$STAMP"
  echo "INSTALLED $APP_DIR/$APP_NAME"
fi

if [ "${1:-}" = "--open" ] || [ "${2:-}" = "--open" ]; then
  TARGET="$APP_SRC"
  [ -d "$APP_DIR/$(basename "$APP_SRC")" ] && TARGET="$APP_DIR/$(basename "$APP_SRC")"
  open "$TARGET"
  echo "OPENED $TARGET"
fi
