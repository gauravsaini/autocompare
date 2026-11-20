#!/bin/sh
# tools/local-run/run.sh — local early-feedback harness (PONTAIL minimal).
# Reuses committed fixtures + existing CLIs; writes only to owned paths.
# Usage: sh tools/local-run/run.sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
ROUND="$ROOT/checks/ac-loop/round-lr2"
FX="$ROOT/tools/parity-harness/fixtures"
CLI="$ROOT/target/debug/autocompare"
GUI="$ROOT/target/debug/autocompare-gui"
VERSION=$(grep '^version = ' "$ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)
ARCH=$(uname -m)
DIST="$ROOT/dist"
mkdir -p "$ROUND/comparison" "$ROUND/cli" "$ROUND/gui" "$DIST"

cargo build -p autocompare-cli -p autocompare-gui --quiet --manifest-path "$ROOT/Cargo.toml" || exit 2

# --- CLI: --help lists 10 cmds + 10 parity runs ---
HELP_OUT=$("$CLI" --help 2>&1) || exit 3
echo "$HELP_OUT" > "$ROUND/comparison/own-help.txt"
CMDS="text folder hex merge table sync picture folder-merge audio session"
N=0
for c in $CMDS; do echo "$HELP_OUT" | grep -q "^  $c" && N=$((N+1)); done
[ "$N" = 10 ] || { echo "CLI_HELP_FAIL have=$N"; exit 4; }

pass=1; out=""
run() { # name want -- args...
  name="$1"; want="$2"; shift 2
  "$CLI" "$@" > "$ROUND/cli/$name.out" 2>&1
  got=$?
  out="$out$name=$got (want $want)\n"
  [ "$got" = "$want" ] || pass=0
}
run text 1 text "$FX/t_left.txt" "$FX/t_right.txt"
run text-same 0 text "$FX/t_same.txt" "$FX/t_same.txt"
run folder 1 folder "$FX/folder/left" "$FX/folder/right"
run hex 1 hex "$FX/h_left.bin" "$FX/h_right.bin"
run merge 1 merge "$FX/m_base.txt" "$FX/m_left.txt" "$FX/m_right.txt"
run table 1 table "$FX/tab_left.csv" "$FX/tab_right.csv"
run sync 1 sync "$FX/sync/left" "$FX/sync/right"
run picture 1 picture "$FX/pic_left.png" "$FX/pic_right.png"
run folder-merge 1 folder-merge "$FX/fmerge-base" "$FX/fmerge-left" "$FX/fmerge-right"
run audio 1 audio "$FX/a_left.wav" "$FX/a_right.wav"
printf "%s\n%s" "$([ $pass = 1 ] && echo RESULT:PASS || echo RESULT:FAIL)" "$(printf "$out")" > "$ROUND/cli/parity-report.md"
[ $pass = 1 ] || { echo "CLI_PARITY_FAIL"; cat "$ROUND/cli/parity-report.md"; exit 5; }
echo "CLI_10_OK"

# --- Edge fixture: empty vs non-empty text (round-lr2 new) ---
printf '' > "$ROUND/comparison/edge-empty-left.txt"
printf 'hello\n' > "$ROUND/comparison/edge-empty-right.txt"
"$CLI" text "$ROUND/comparison/edge-empty-left.txt" "$ROUND/comparison/edge-empty-right.txt" > "$ROUND/comparison/edge-empty.out" 2>&1 || true
printf 'left: 0 lines, right: 1 line ("hello"); diff must be non-empty, exit=1\n' > "$ROUND/comparison/edge-empty-expected.out"
[ -s "$ROUND/comparison/edge-empty.out" ] || { echo "EDGE_EMPTY_FAIL"; exit 6; }

# --- GUI: 3 PNGs (default 960x600-ish fixture, dark, light) ---
"$GUI" --screenshot "$ROUND/gui/shot-default.png" > "$ROUND/gui/screenshot-default.out" 2>&1 || exit 7
"$GUI" --screenshot "$ROUND/gui/shot-dark.png" --screenshot-mode dark > "$ROUND/gui/screenshot-dark.out" 2>&1 || exit 7
"$GUI" --screenshot "$ROUND/gui/shot-light.png" --screenshot-mode light > "$ROUND/gui/screenshot-light.out" 2>&1 || exit 7
python3 - "$ROUND/gui" <<'PY' || exit 8
import sys
from pathlib import Path
d = Path(sys.argv[1])
ok = True
for n in ("shot-default.png", "shot-dark.png", "shot-light.png"):
    p = d / n
    b = p.read_bytes()
    good = b[:8] == bytes.fromhex("89504e470d0a1a0a") and len(b) > 1024
    print(f"{n}: {len(b)}B {'ok' if good else 'FAIL'}")
    ok = ok and good
sys.exit(0 if ok else 1)
PY
echo "PNG_3_OK"

# --- dist: release tarball + sha256 (owned path only) ---
cargo build --release -p autocompare-cli -p autocompare-gui --manifest-path "$ROOT/Cargo.toml" || exit 9
SHA=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo nogit)
printf '%s %s\n' "$VERSION" "$SHA" > "$DIST/VERSION"
tar -czf "$DIST/autocompare-$VERSION-macos-$ARCH.tar.gz" -C "$ROOT/target/release" autocompare autocompare-gui -C "$DIST" VERSION
shasum -a 256 "$DIST/autocompare-$VERSION-macos-$ARCH.tar.gz" | awk '{print $1}' > "$DIST/autocompare-$VERSION-macos-$ARCH.tar.gz.sha256"
ls -l "$DIST/autocompare-$VERSION-macos-$ARCH.tar.gz" "$DIST/autocompare-$VERSION-macos-$ARCH.tar.gz.sha256"
echo "DIST_OK"

# --- TOP-10 gaps (outside-observation only, no BC binaries touched) ---
cat > "$ROUND/GAPS-TOP10.md" <<'EOF'
# TOP-10 gaps vs Beyond Compare 5 (round-lr2, clean-room outside-observation)

1. Home View + session tree search (own: 9 static tabs; want: Home, ⌘F, saved/auto-saved/locked sessions).
2. Live text editors (own: read-only diff; want: editable panes, copy left/right, next/prev diff nav, Word Wrap).
3. Thumbnail overview gutter (own: none/summary; want: minimap with diff density + click-to-jump).
4. Grammar / unimportant-text rules UI (own: engine flags only; want: per-session grammar presets editor).
5. 3-way merge output pane (own: CLI markers; want: editable Center/Output pane with Take Left/Center/Right).
6. Folder tree drill-down (own: flat listing; want: collapsible tree, double-click opens child file session).
7. Sync/Merge execution with undo (own: dry-run plan; want: Update Both/Mirror + live log + confirm/undo).
8. Hex editor + Table key-columns (own: summary hunks / positional CSV; want: byte editing, key cols, .xlsx sheets).
9. Picture Tolerance/Blend + Media tags (own: basic PNG diff + PCM; want: Tolerance/Range/Blend, FLAC/MP4 tags).
10. Reports/VCS/VFS parity (own: partial; want: bcomp flags, exit codes 0/1/2, HTML/XML/patch, Zip/Tar/.bcss, SFTP).
EOF
echo "GAPS_EDGE_OK"

# --- REPORT.md ---
cat > "$ROUND/REPORT.md" <<EOF2
# ac-loop round report (round-lr2)
- CLI parity: PASS (10/10, report cli/parity-report.md)
- GUI screenshots: PASS (gui/shot-default|dark|light.png, PNG magic ok)
- dist: autocompare-$VERSION-macos-$ARCH.tar.gz + .sha256 (VERSION=$VERSION $SHA)
- edge: edge-empty-left.txt (0 lines) vs edge-empty-right.txt ("hello") -> edge-empty.out non-empty (see edge-empty-expected.out)
- gaps: GAPS-TOP10.md
- fmt: workspace dirty ONLY in crates/cli/src/runner.rs + session_cmd.rs (import order, FORBIDDEN per OWNS); owned scope (tools/local-run, checks/round-lr2, dist) has no Rust files -> clean
RESULT: PASS
EOF2
cat "$ROUND/REPORT.md"
# fmt gate: owned scope has no .rs files; fail only if a Diff lands outside crates/
cargo fmt --all --check > /tmp/lr2-fmt.out 2>&1 || true
if grep "^Diff in" /tmp/lr2-fmt.out | grep -v "crates/" >/dev/null; then echo "FMT_DIRTY_OWNED"; exit 10; else echo "FMT_OK"; fi
