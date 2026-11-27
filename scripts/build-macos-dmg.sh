#!/usr/bin/env bash
# AutoCompare — Clean, deterministic macOS DMG packager (Tauri v2 + native hdiutil).
#
# Why this script instead of Tauri's default Finder-AppleScript DMG step?
#   - Never hangs on Finder Automation permissions or leaves stale `rw.*.dmg` files.
#   - Works reliably both locally and in headless CI.
#   - Builds the v2 UI (`VITE_UI_VERSION=v2`), ad-hoc signs (or uses APPLE_SIGNING_IDENTITY),
#     packages `AutoCompare.app` + `/Applications` symlink into a compressed UDZO DMG,
#     and emits a `.sha256` checksum.
#
# Usage:
#   scripts/build-macos-dmg.sh [--arch native|aarch64|x86_64|universal] [--skip-tests] [--open]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DESK="$ROOT/apps/desktop"
VERSION="$(grep '^version = ' "$ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)"
HOST_ARCH="$(uname -m)"
ARCH_MODE="native"
SKIP_TESTS=0
OPEN_AFTER=0

while [ $# -gt 0 ]; do
  case "$1" in
    --arch)
      ARCH_MODE="${2:-native}"
      shift 2
      ;;
    --skip-tests)
      SKIP_TESTS=1
      shift
      ;;
    --open)
      OPEN_AFTER=1
      shift
      ;;
    -h|--help)
      sed -n '2,14p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 2
      ;;
  esac
done

TARGET_ARGS=()
OUT_ARCH="$HOST_ARCH"
case "$ARCH_MODE" in
  native)
    ;;
  aarch64|arm64)
    TARGET_ARGS=(--target aarch64-apple-darwin)
    OUT_ARCH="aarch64"
    ;;
  x86_64)
    TARGET_ARGS=(--target x86_64-apple-darwin)
    OUT_ARCH="x86_64"
    ;;
  universal)
    TARGET_ARGS=(--target universal-apple-darwin)
    OUT_ARCH="universal"
    ;;
  *)
    echo "Unsupported --arch value: $ARCH_MODE (use native, aarch64, x86_64, or universal)" >&2
    exit 2
    ;;
esac

TAURI_BIN="$(command -v cargo-tauri 2>/dev/null || echo "$HOME/.cargo/bin/cargo-tauri")"
if [ ! -x "$TAURI_BIN" ]; then
  echo "ERROR: cargo-tauri not found at $TAURI_BIN (install via: cargo install tauri-cli)" >&2
  exit 1
fi

# 1. Clean stale leftover rw.*.dmg artifacts from previous interrupted Tauri runs
rm -f "$ROOT"/target/release/bundle/macos/rw.*.dmg \
      "$DESK"/src-tauri/target/release/bundle/macos/rw.*.dmg 2>/dev/null || true

# 2. Ensure frontend deps are installed
if [ ! -x "$DESK/node_modules/.bin/tsc" ]; then
  echo "==> Installing desktop frontend dependencies..."
  (cd "$DESK" && npm ci --no-audit --no-fund)
fi

# 3. Run fast unit tests + build production v2 frontend
if [ "$SKIP_TESTS" -eq 0 ]; then
  echo "==> Running v2 unit tests..."
  (cd "$DESK" && npm test)
fi

echo "==> Building AutoCompare v2 frontend..."
(cd "$DESK" && VITE_UI_VERSION=v2 npm run build)

# 4. Build macOS .app bundle via Tauri (skipping flaky AppleScript DMG step)
echo "==> Building AutoCompare.app ($OUT_ARCH)..."
(
  cd "$DESK"
  TAURI_SKIP_UPDATE_CHECK=1 "$TAURI_BIN" tauri build \
    --bundles app --ci \
    "${TARGET_ARGS[@]}" \
    -c '{"build": {"beforeBuildCommand": ""}}'
)

APP_BUNDLE=""
for candidate in \
  "$ROOT/target/release/bundle/macos/AutoCompare.app" \
  "$ROOT/target/${OUT_ARCH}-apple-darwin/release/bundle/macos/AutoCompare.app" \
  "$ROOT/target/universal-apple-darwin/release/bundle/macos/AutoCompare.app" \
  "$DESK/src-tauri/target/release/bundle/macos/AutoCompare.app"; do
  if [ -d "$candidate" ]; then
    APP_BUNDLE="$candidate"
    break
  fi
done

if [ -z "$APP_BUNDLE" ]; then
  echo "ERROR: AutoCompare.app bundle not found after Tauri build" >&2
  exit 1
fi
echo "==> Found bundle: $APP_BUNDLE"

# 5. Sign app bundle (Developer ID if APPLE_SIGNING_IDENTITY is set, else clean ad-hoc)
SIGN_ID="${APPLE_SIGNING_IDENTITY:--}"
echo "==> Signing AutoCompare.app (identity: $SIGN_ID)..."
xattr -cr "$APP_BUNDLE" 2>/dev/null || true
codesign --force --deep --sign "$SIGN_ID" "$APP_BUNDLE"

# 6. Stage DMG contents with /Applications drag-install symlink
STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/autocompare-dmg.XXXXXX")"
trap 'rm -rf "$STAGE_DIR"' EXIT

cp -R "$APP_BUNDLE" "$STAGE_DIR/AutoCompare.app"
ln -s /Applications "$STAGE_DIR/Applications"

OUT_DIR="$ROOT/dist"
mkdir -p "$OUT_DIR"
DMG_PATH="$OUT_DIR/AutoCompare-${VERSION}-macos-${OUT_ARCH}.dmg"
rm -f "$DMG_PATH" "${DMG_PATH}.sha256"

echo "==> Creating compressed UDZO DMG at $DMG_PATH..."
hdiutil create \
  -volname "AutoCompare ${VERSION}" \
  -srcfolder "$STAGE_DIR" \
  -ov \
  -format UDZO \
  -imagekey zlib-level=9 \
  "$DMG_PATH"

# 7. Emit SHA-256 checksum
(cd "$OUT_DIR" && shasum -a 256 "$(basename "$DMG_PATH")" > "$(basename "$DMG_PATH").sha256")

DMG_SIZE="$(du -h "$DMG_PATH" | awk '{print $1}')"
echo ""
echo "DMG_OK: $DMG_PATH ($DMG_SIZE)"
echo "SHA256: $(cat "${DMG_PATH}.sha256")"

if [ "$OPEN_AFTER" -eq 1 ]; then
  open "$DMG_PATH"
fi
