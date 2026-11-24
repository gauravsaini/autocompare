#!/bin/sh
# BINARY wave pack: build release binaries, smoke, stage dist/ tarballs.
# Usage: sh scripts/release-early-pack.sh [--check-only]
# --check-only: verify dist freshness only, no build.
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
VERSION=$(grep '^version = ' "$ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)
SHA=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)
ARCH=$(uname -m)
DIST="$ROOT/dist"

if [ "${1:-}" = "--check-only" ]; then
  ok=1
  for a in "autocompare-cli-$VERSION-macos-$ARCH.tar.gz" "autocompare-gui-$VERSION-macos-$ARCH.tar.gz"; do
    [ -s "$DIST/$a" ] || { echo "MISSING $a"; ok=0; }
  done
  [ -f "$DIST/VERSION" ] && { [ -f "$ROOT/checks/bin-early/pin.env" ] && PIN_WANT=$(cut -d= -f2 "$ROOT/checks/bin-early/pin.env") || PIN_WANT="$SHA"; grep -q "$PIN_WANT" "$DIST/VERSION"; } || { echo "VERSION_STALE (want $PIN_WANT)"; ok=0; }
  [ "$ok" = 1 ] && { echo "DIST_OK"; exit 0; } || exit 1
fi

cargo build --release -p autocompare-cli -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" || exit 2

# smoke via bin-early harness
sh "$ROOT/checks/bin-early/run-smoke.sh" || exit 3

mkdir -p "$DIST"
printf '%s %s\n' "$VERSION" "$SHA" > "$DIST/VERSION"
tar -czf "$DIST/autocompare-cli-$VERSION-macos-$ARCH.tar.gz" \
  -C "$ROOT/target/release" autocompare -C "$DIST" VERSION
tar -czf "$DIST/autocompare-gui-$VERSION-macos-$ARCH.tar.gz" \
  -C "$ROOT/target/release" autocompare-gui -C "$DIST" VERSION
ls -l "$DIST"
echo "DIST_OK"
