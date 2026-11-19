#!/bin/sh
# macOS early-feedback install path for autocompare-gui — no cargo needed
# to USE the result, only to BUILD it once here.
# Usage: scripts/install-macos.sh [--install]  (PREFIX env, default $HOME/.local/bin)
# Output: dist/autocompare-gui-<version>-macos-<arch>.tar.gz (binary + VERSION stamp)
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN_PKG=autocompare-gui
VERSION=$(grep '^version = ' "$ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)
SHA=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)
ARCH=$(uname -m)
DIST="$ROOT/dist"
BIN="$ROOT/target/release/$BIN_PKG"

cargo build --release -p "$BIN_PKG"

# smoke: offscreen fixture render, no window
SMOKE_TMP=$(mktemp -t ac-smoke-XXXXXX).png
"$BIN" --screenshot "$SMOKE_TMP"
[ -s "$SMOKE_TMP" ] || { echo "SMOKE_FAIL empty png"; exit 1; }
rm -f "$SMOKE_TMP"

mkdir -p "$DIST"
printf '%s %s\n' "$VERSION" "$SHA" > "$DIST/VERSION"
TARBALL="$DIST/$BIN_PKG-$VERSION-macos-$ARCH.tar.gz"
tar -czf "$TARBALL" -C "$(dirname "$BIN")" "$BIN_PKG" -C "$DIST" VERSION
ls -l "$TARBALL"

if [ "${1:-}" = "--install" ]; then
  PREFIX="${PREFIX:-$HOME/.local/bin}"
  mkdir -p "$PREFIX"
  cp "$BIN" "$PREFIX/"
  cp "$DIST/VERSION" "$PREFIX/autocompare-gui.VERSION"
  echo "INSTALLED $PREFIX/$BIN_PKG"
fi
