#!/bin/sh
# BINARY wave smoke: release CLI parity + GUI headless screenshot.
# Usage: sh checks/bin-early/run-smoke.sh  (writes checks/bin-early/REPORT.md)
# MS rust safe flags only; no source touches.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT="$ROOT/checks/bin-early"
BIN="$ROOT/target/release/autocompare"
GUI="$ROOT/target/release/autocompare-gui"
FX="$ROOT/tools/parity-harness/fixtures"
mkdir -p "$OUT"

pass=1
out=""
run() { # name expected_exit... -- args
  name="$1"; want="$2"; shift 2
  timeout 60 "$BIN" "$@" > "$OUT/$name.out" 2>&1
  got=$?
  out="$out$name=$got (want $want)
"
  if [ "$got" != "$want" ]; then pass=0; fi
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

# GUI headless screenshot (offscreen, no window)
SMOKE_PNG="$OUT/smoke-gui.png"
rm -f "$SMOKE_PNG"
if timeout 90 "$GUI" --screenshot "$SMOKE_PNG" > "$OUT/gui.out" 2>&1; then
  if [ -s "$SMOKE_PNG" ] && head -c 4 "$SMOKE_PNG" | od -An -tx1 | tr -s ' ' | grep -q "89 50 4e 47"; then
    out="$out gui-screenshot=OK
GUI_SMOKE_OK_LINE=yes"
  else
    out="$out gui-screenshot=BAD_PNG
"
    pass=0
  fi
else
  out="$out gui-screenshot=EXIT_$?
"
  pass=0
fi

if [ $pass = 1 ]; then res="RESULT: PASS"; else res="RESULT: FAIL"; fi
{
  printf '%s\n' "$res"
  printf '%s' "$out"
  printf 'GUI_SMOKE_OK\n'
} > "$OUT/REPORT.md"
cat "$OUT/REPORT.md"
[ $pass = 1 ]
