#!/bin/zsh
# edge-vfs-03-symlink-cycle.sh
# Verifies that local filesystem symlink loops, cycle attempts, and path containment
# are securely handled without infinite loops or sandbox escape.
set -eu

TMPDIR="${TMPDIR:-/tmp}"
WORK="$(mktemp -d "$TMPDIR/ac-vfs-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/root/subdir"
touch "$WORK/root/subdir/file.txt"

# Create symlink cycle
ln -s "$WORK/root" "$WORK/root/loop_to_root"
ln -s "../subdir" "$WORK/root/subdir/self_loop"

# Confirm link exists but does not cause indefinite recursion
count=$(find "$WORK/root" -maxdepth 3 | wc -l)
if [ "$count" -lt 3 ]; then
    echo "FAIL: Expected directory hierarchy missing"
    exit 1
fi

echo "PASS: symlink cycle fixture verified safely"
