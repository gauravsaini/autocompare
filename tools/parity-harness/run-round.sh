#!/bin/zsh
# Parity-harness one-round check: 9 CLI diff-convention runs on committed fixtures.
# Usage: run-round.sh <round-dir>   (writes <round-dir>/parity-report.md, exits 0 iff PASS)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run-round.sh <round-dir>}"
FX="$ROOT/tools/parity-harness/fixtures"
BIN="$ROOT/target/debug/autocompare"
mkdir -p "$ROUND"
cargo build -p autocompare-cli --quiet --manifest-path "$ROOT/Cargo.toml" || exit 2
pass=1
out=""
run() { # name expected_exit... -- args
  local name="$1"; local want="$2"; shift 2
  "$BIN" "$@" > "$ROUND/$name.out" 2>&1
  local got=$?
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
if [ $pass = 1 ]; then res="RESULT: PASS"; else res="RESULT: FAIL"; fi
printf "%s\n%s" "$res" "$(printf "$out")" > "$ROUND/parity-report.md"
cat "$ROUND/parity-report.md"
[ $pass = 1 ]
