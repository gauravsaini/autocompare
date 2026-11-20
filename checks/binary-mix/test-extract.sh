#!/bin/sh
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TMPDIR="/tmp/test-extract-binmix-$$"
mkdir -p "$TMPDIR"
trap 'rm -rf "$TMPDIR"' EXIT

TARBALL="$ROOT/dist/autocompare-0.5.0-macos-arm64.tar.gz"
if [ ! -f "$TARBALL" ]; then
  echo "Tarball missing: $TARBALL"
  exit 1
fi

tar -xzf "$TARBALL" -C "$TMPDIR"
if [ -x "$TMPDIR/autocompare" ] && [ -x "$TMPDIR/autocompare-gui" ]; then
  "$TMPDIR/autocompare" --help > /dev/null
  echo "EXTRACT_OK"
  exit 0
fi

echo "Extracted binaries missing or not executable"
exit 2
