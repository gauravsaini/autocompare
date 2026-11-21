#!/usr/bin/env bash
# Release build for autocompare-gui. Tmp-only, no network beyond cargo.
set -euo pipefail
cd "$(dirname "$0")/../.."
export TMPDIR="${TMPDIR:-/tmp}"
cargo build --release -p autocompare-gui
BIN="./target/release/autocompare-gui"
test -x "$BIN"
SIZE=$(wc -c < "$BIN" | tr -d ' ')
echo "binary: $BIN (${SIZE} bytes)"
echo "RELEASE_BUILD_OK"
